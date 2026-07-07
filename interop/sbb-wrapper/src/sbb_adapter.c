#include "sbb_adapter.h"
#include "sbb_diagnostics.h"
#include "udp_transport.h"

#include <stdio.h>
#include <string.h>

#ifdef SBB_WRAPPER_HAS_SBB_REDL
#include "rasta_redundancy/redcty_red_config_types.h"
#include "rasta_redundancy/redint_red_interface.h"
#include "rasta_redundancy/redtrn_transport_notifications.h"

static redcty_RedundancyLayerConfiguration g_redl_config = {
    .check_code_type = redcty_kCheckCodeA,
    .t_seq = 50U,
    .n_diagnosis = 200U,
    .n_defer_queue_size = 4U,
    .number_of_redundancy_channels = 2U,
    .redundancy_channel_configurations = {
        {
            .red_channel_id = 0U,
            .num_transport_channels = 2U,
            .transport_channel_ids = {0U, 1U},
        },
        {
            .red_channel_id = 1U,
            .num_transport_channels = 2U,
            .transport_channel_ids = {2U, 3U},
        },
    },
};
#endif

#define SBB_WRAPPER_TRANSPORT_CHANNEL_COUNT 2u
#define SBB_WRAPPER_REDUNDANCY_CHANNEL_COUNT 2u
#define SBB_WRAPPER_SR_DISC_REQ 6216u

typedef struct SbbWrapperPendingDatagram {
    int occupied;
    uint16_t length;
    uint8_t bytes[RADEF_MAX_RED_LAYER_PDU_MESSAGE_SIZE];
} SbbWrapperPendingDatagram;

static SbbWrapperPendingDatagram g_pending[SBB_WRAPPER_TRANSPORT_CHANNEL_COUNT];
static int g_red_channel_open[SBB_WRAPPER_REDUNDANCY_CHANNEL_COUNT];
static unsigned int g_red_read_allowed[SBB_WRAPPER_REDUNDANCY_CHANNEL_COUNT];
static int g_disconnect_frame_in_progress = 0;
static int g_disconnect_red_read_consumed = 0;
static int g_redl_initialized = 0;

static int valid_red_channel(uint32_t redundancy_channel_id)
{
    return redundancy_channel_id < SBB_WRAPPER_REDUNDANCY_CHANNEL_COUNT;
}

static SbbWrapperPendingDatagram *pending_slot(uint32_t transport_channel_id)
{
    if (transport_channel_id >= SBB_WRAPPER_TRANSPORT_CHANNEL_COUNT) {
        return 0;
    }
    return &g_pending[transport_channel_id];
}

static void clear_pending_slots(void)
{
    uint32_t i;
    for (i = 0u; i < SBB_WRAPPER_TRANSPORT_CHANNEL_COUNT; i += 1u) {
        g_pending[i].occupied = 0;
        g_pending[i].length = 0u;
    }
}

static void clear_red_channel_state(void)
{
    uint32_t i;
    for (i = 0u; i < SBB_WRAPPER_REDUNDANCY_CHANNEL_COUNT; i += 1u) {
        g_red_channel_open[i] = 0;
        g_red_read_allowed[i] = 0u;
    }
    g_disconnect_frame_in_progress = 0;
    g_disconnect_red_read_consumed = 0;
}

int sbb_wrapper_redl_begin_message_notification(uint32_t redundancy_channel_id)
{
    if (!valid_red_channel(redundancy_channel_id)) {
        return 0;
    }
    if (sbb_wrapper_diag_closed_after_up()) {
        if (sbb_wrapper_udp_trace_enabled()) {
            printf(
                "[sbb-wrapper] RedL read allowance skipped: channel=%u connection already closed after Up\n",
                redundancy_channel_id);
        }
        return 0;
    }
    if (g_disconnect_frame_in_progress && g_disconnect_red_read_consumed) {
        if (sbb_wrapper_udp_trace_enabled()) {
            printf(
                "[sbb-wrapper] RedL read allowance skipped: channel=%u DiscReq read already consumed\n",
                redundancy_channel_id);
        }
        return 0;
    }
    g_red_read_allowed[redundancy_channel_id] += 1u;
    if (sbb_wrapper_udp_trace_enabled()) {
        printf(
            "[sbb-wrapper] RedL read allowance begin: channel=%u allowance=%u\n",
            redundancy_channel_id,
            g_red_read_allowed[redundancy_channel_id]);
    }
    return 1;
}

void sbb_wrapper_redl_end_message_notification(uint32_t redundancy_channel_id)
{
    if (!valid_red_channel(redundancy_channel_id)) {
        return;
    }
    g_red_read_allowed[redundancy_channel_id] = 0u;
    if (sbb_wrapper_udp_trace_enabled()) {
        printf("[sbb-wrapper] RedL read allowance end: channel=%u\n", redundancy_channel_id);
    }
}

static uint16_t read_le_u16(const uint8_t *bytes)
{
    return (uint16_t)((uint16_t)bytes[0] | ((uint16_t)bytes[1] << 8));
}

static uint16_t decode_sr_type(const uint8_t *bytes, uint16_t length)
{
    if (length < (RADEF_RED_LAYER_MESSAGE_HEADER_SIZE + 4u)) {
        return 0u;
    }
    return read_le_u16(&bytes[RADEF_RED_LAYER_MESSAGE_HEADER_SIZE + 2u]);
}

static void note_received_sr_type(uint16_t sr_type)
{
    sbb_wrapper_diag_observe_sr_type(sr_type);
    if (sr_type == SBB_WRAPPER_SR_DISC_REQ) {
        g_disconnect_frame_in_progress = 1;
        g_disconnect_red_read_consumed = 0;
        if (sbb_wrapper_udp_trace_enabled()) {
            puts("[sbb-wrapper] DiscReq detected; limiting RedL read handling for this notification");
        }
    } else if (!sbb_wrapper_diag_closed_after_up()) {
        g_disconnect_frame_in_progress = 0;
        g_disconnect_red_read_consumed = 0;
    }
}

static const char *sr_message_type_name(uint16_t message_type)
{
    switch (message_type) {
    case 6200u:
        return "ConnReq";
    case 6201u:
        return "ConnResp";
    case 6212u:
        return "RetrReq";
    case 6213u:
        return "RetrResp";
    case 6216u:
        return "DiscReq";
    case 6220u:
        return "Heartbeat";
    case 6240u:
        return "Data";
    case 6241u:
        return "RetrData";
    default:
        return "Unknown";
    }
}

static void log_received_red_frame(uint32_t transport_channel_id, const uint8_t *bytes, uint16_t length)
{
    uint16_t red_length = 0u;
    uint16_t sr_length = 0u;
    uint16_t sr_type = 0u;
    uint16_t i;
    uint16_t prefix_len = length < 16u ? length : 16u;

    if (!sbb_wrapper_udp_trace_enabled()) {
        return;
    }

    if (length >= 2u) {
        red_length = read_le_u16(bytes);
    }
    if (length >= (RADEF_RED_LAYER_MESSAGE_HEADER_SIZE + 4u)) {
        sr_length = read_le_u16(&bytes[RADEF_RED_LAYER_MESSAGE_HEADER_SIZE]);
        sr_type = decode_sr_type(bytes, length);
    }

    printf(
        "[sbb-wrapper] received RedL frame before notification: transport=%u source=%s:%u datagram_length=%u red_length=%u sr_length=%u sr_type=0x%04x(%s) prefix=",
        transport_channel_id,
        sbb_wrapper_udp_last_receive_ip(),
        (unsigned int)sbb_wrapper_udp_last_receive_port(),
        length,
        red_length,
        sr_length,
        sr_type,
        sr_message_type_name(sr_type));
    for (i = 0u; i < prefix_len; i += 1u) {
        printf("%02x", bytes[i]);
    }
    puts("");
}

void sradin_Init(void)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result = redint_Init(&g_redl_config);
    g_redl_initialized = (result == radef_kNoError || result == radef_kAlreadyInitialized);
    clear_red_channel_state();
    printf(
        "[sbb-wrapper] sradin_Init: redint_Init result=%u(%s)\n",
        (unsigned int)result,
        sbb_wrapper_rasta_return_code_name(result));
#else
    g_redl_initialized = 0;
    clear_red_channel_state();
    puts("[sbb-wrapper] sradin_Init: SBB RedL not linked");
#endif
}

void sradin_OpenRedundancyChannel(uint32_t redundancy_channel_id)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result = redint_OpenRedundancyChannel(redundancy_channel_id);
    if (result == radef_kNoError || result == radef_kAlreadyInitialized) {
        if (valid_red_channel(redundancy_channel_id)) {
            g_red_channel_open[redundancy_channel_id] = 1;
        }
    }
    printf(
        "[sbb-wrapper] sradin_OpenRedundancyChannel: channel=%u result=%u(%s)\n",
        redundancy_channel_id,
        (unsigned int)result,
        sbb_wrapper_rasta_return_code_name(result));
#else
    printf("[sbb-wrapper] sradin_OpenRedundancyChannel: channel=%u SBB RedL not linked\n", redundancy_channel_id);
#endif
}

void sradin_CloseRedundancyChannel(uint32_t redundancy_channel_id)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result = redint_CloseRedundancyChannel(redundancy_channel_id);
    if (valid_red_channel(redundancy_channel_id)) {
        g_red_channel_open[redundancy_channel_id] = 0;
        g_red_read_allowed[redundancy_channel_id] = 0u;
    }
    printf(
        "[sbb-wrapper] sradin_CloseRedundancyChannel: channel=%u result=%u(%s)\n",
        redundancy_channel_id,
        (unsigned int)result,
        sbb_wrapper_rasta_return_code_name(result));
#else
    printf("[sbb-wrapper] sradin_CloseRedundancyChannel: channel=%u SBB RedL not linked\n", redundancy_channel_id);
#endif
}

void sradin_SendMessage(uint32_t redundancy_channel_id, uint16_t message_size, const uint8_t *message_data)
{
    if (message_data == 0 && message_size != 0u) {
        puts("[sbb-wrapper] sradin_SendMessage: invalid null message");
        return;
    }

#ifdef SBB_WRAPPER_HAS_SBB_REDL
    {
        sbb_wrapper_diag_set_phase("sradin_SendMessage:redint_SendMessage");
        radef_RaStaReturnCode result = redint_SendMessage(redundancy_channel_id, message_size, message_data);
        printf(
            "[sbb-wrapper] sradin_SendMessage: channel=%u length=%u redint_SendMessage result=%u(%s)\n",
            redundancy_channel_id,
            message_size,
            (unsigned int)result,
            sbb_wrapper_rasta_return_code_name(result));
    }
#else
    printf(
        "[sbb-wrapper] sradin_SendMessage: channel=%u length=%u SBB RedL not linked\n",
        redundancy_channel_id,
        message_size);
#endif
}

radef_RaStaReturnCode sradin_ReadMessage(
    uint32_t redundancy_channel_id,
    uint16_t buffer_size,
    uint16_t *message_size,
    uint8_t *message_buffer)
{
    if (message_size != 0) {
        *message_size = 0u;
    }

#ifdef SBB_WRAPPER_HAS_SBB_REDL
    {
        if (!valid_red_channel(redundancy_channel_id)) {
            printf(
                "[sbb-wrapper] sradin_ReadMessage: channel=%u skipped because redundancy channel is invalid\n",
                redundancy_channel_id);
            return radef_kNoMessageReceived;
        }
        if (!g_red_channel_open[redundancy_channel_id]) {
            if (sbb_wrapper_udp_trace_enabled()) {
                printf(
                    "[sbb-wrapper] sradin_ReadMessage: channel=%u skipped because RedL channel is not open\n",
                    redundancy_channel_id);
            }
            return radef_kNoMessageReceived;
        }
        if (sbb_wrapper_diag_closed_after_up()) {
            if (sbb_wrapper_udp_trace_enabled()) {
                printf(
                    "[sbb-wrapper] sradin_ReadMessage: channel=%u skipped because connection closed after Up\n",
                    redundancy_channel_id);
            }
            return radef_kNoMessageReceived;
        }
        if (g_red_read_allowed[redundancy_channel_id] == 0u) {
            if (sbb_wrapper_udp_trace_enabled()) {
                printf(
                    "[sbb-wrapper] sradin_ReadMessage: channel=%u skipped because no RedL message notification is active\n",
                    redundancy_channel_id);
            }
            return radef_kNoMessageReceived;
        }
        g_red_read_allowed[redundancy_channel_id] -= 1u;
        if (g_disconnect_frame_in_progress) {
            if (g_disconnect_red_read_consumed) {
                if (sbb_wrapper_udp_trace_enabled()) {
                    printf(
                        "[sbb-wrapper] sradin_ReadMessage: channel=%u skipped because DiscReq read was already consumed\n",
                        redundancy_channel_id);
                }
                return radef_kNoMessageReceived;
            }
            g_disconnect_red_read_consumed = 1;
            if (sbb_wrapper_udp_trace_enabled()) {
                printf(
                    "[sbb-wrapper] sradin_ReadMessage: channel=%u consuming one allowed DiscReq RedL read\n",
                    redundancy_channel_id);
            }
        }
        sbb_wrapper_diag_set_phase("sradin_ReadMessage:redint_CheckTimings");
        radef_RaStaReturnCode timing_result = redint_CheckTimings();
        if (sbb_wrapper_diag_closed_after_up()) {
            if (sbb_wrapper_udp_trace_enabled()) {
                printf(
                    "[sbb-wrapper] sradin_ReadMessage: channel=%u redint_ReadMessage skipped after redint_CheckTimings because connection closed after Up\n",
                    redundancy_channel_id);
            }
            return radef_kNoMessageReceived;
        }
        sbb_wrapper_diag_set_phase("sradin_ReadMessage:redint_ReadMessage");
        if (sbb_wrapper_udp_trace_enabled()) {
            printf(
                "[sbb-wrapper] sradin_ReadMessage: channel=%u calling redint_ReadMessage allowance_remaining=%u\n",
                redundancy_channel_id,
                g_red_read_allowed[redundancy_channel_id]);
        }
        radef_RaStaReturnCode read_result = redint_ReadMessage(redundancy_channel_id, buffer_size, message_size, message_buffer);
        printf(
            "[sbb-wrapper] sradin_ReadMessage: channel=%u timing_result=%u(%s) read_result=%u(%s) length=%u\n",
            redundancy_channel_id,
            (unsigned int)timing_result,
            sbb_wrapper_rasta_return_code_name(timing_result),
            (unsigned int)read_result,
            sbb_wrapper_rasta_return_code_name(read_result),
            message_size == 0 ? 0u : *message_size);
        return read_result;
    }
#else
    printf(
        "[sbb-wrapper] sradin_ReadMessage: channel=%u SBB RedL not linked\n",
        redundancy_channel_id);
    return radef_kNoMessageReceived;
#endif
}

void redtri_Init(void)
{
    if (!sbb_wrapper_udp_is_initialized()) {
        puts("[sbb-wrapper] redtri_Init: UDP transport is not initialized");
        return;
    }

    clear_pending_slots();
    clear_red_channel_state();
    g_redl_initialized = 0;
    puts("[sbb-wrapper] redtri_Init: UDP transport ready");
}

void redtri_SendMessage(uint32_t transport_channel_id, uint16_t message_size, const uint8_t *message_data)
{
    SbbWrapperUdpResult result;

    if (message_data == 0 && message_size != 0u) {
        puts("[sbb-wrapper] redtri_SendMessage: invalid null message");
        return;
    }

    sbb_wrapper_diag_set_phase("redtri_SendMessage:udp_send");
    result = sbb_wrapper_udp_send(transport_channel_id, message_data, message_size);
    if (result == SBB_WRAPPER_UDP_OK) {
        printf(
            "[sbb-wrapper] redtri_SendMessage: transport=%u length=%u sent\n",
            transport_channel_id,
            message_size);
        return;
    }

    printf(
        "[sbb-wrapper] redtri_SendMessage: transport=%u length=%u failed udp_result=%d\n",
        transport_channel_id,
        message_size,
        result);
}

radef_RaStaReturnCode redtri_ReadMessage(
    uint32_t transport_channel_id,
    uint16_t buffer_size,
    uint16_t *message_size,
    uint8_t *message_buffer)
{
    SbbWrapperPendingDatagram *slot = pending_slot(transport_channel_id);

    if (message_size != 0) {
        *message_size = 0u;
    }

    if (slot == 0 || message_size == 0 || message_buffer == 0) {
        printf("[sbb-wrapper] redtri_ReadMessage: transport=%u invalid parameter\n", transport_channel_id);
        return radef_kInvalidParameter;
    }

    if (!slot->occupied) {
        printf("[sbb-wrapper] redtri_ReadMessage: transport=%u no message\n", transport_channel_id);
        return radef_kNoMessageReceived;
    }

    if (slot->length > buffer_size) {
        printf(
            "[sbb-wrapper] redtri_ReadMessage: transport=%u pending length=%u exceeds buffer=%u\n",
            transport_channel_id,
            slot->length,
            buffer_size);
        slot->occupied = 0;
        slot->length = 0u;
        return radef_kInvalidBufferSize;
    }

    sbb_wrapper_diag_set_phase("redtri_ReadMessage:consume_pending");
    memcpy(message_buffer, slot->bytes, slot->length);
    *message_size = slot->length;
    slot->occupied = 0;
    slot->length = 0u;

    printf(
        "[sbb-wrapper] redtri_ReadMessage: transport=%u length=%u consumed pending datagram\n",
        transport_channel_id,
        *message_size);
    return radef_kNoError;
}

int sbb_wrapper_transport_poll_channel(uint32_t transport_channel_id)
{
    SbbWrapperPendingDatagram *slot = pending_slot(transport_channel_id);
    SbbWrapperUdpResult result;
    size_t received_length = 0u;

    if (slot == 0) {
        return -1;
    }

    if (sbb_wrapper_diag_closed_after_up()) {
        if (sbb_wrapper_udp_trace_enabled()) {
            printf(
                "[sbb-wrapper] transport poll: channel=%u skipped because connection closed after Up\n",
                transport_channel_id);
        }
        return 0;
    }

    if (slot->occupied) {
        if (sbb_wrapper_udp_trace_enabled()) {
            printf(
                "[sbb-wrapper] transport poll: channel=%u pending length=%u retained\n",
                transport_channel_id,
                slot->length);
        }
        return 0;
    }

    result = sbb_wrapper_udp_receive(
        transport_channel_id,
        slot->bytes,
        sizeof(slot->bytes),
        &received_length);
    if (result == SBB_WRAPPER_UDP_NO_MESSAGE) {
        return 0;
    }
    if (result != SBB_WRAPPER_UDP_OK) {
        printf(
            "[sbb-wrapper] transport poll: channel=%u udp_result=%d\n",
            transport_channel_id,
            result);
        return -1;
    }

    slot->occupied = 1;
    slot->length = (uint16_t)received_length;
    log_received_red_frame(transport_channel_id, slot->bytes, slot->length);
    note_received_sr_type(decode_sr_type(slot->bytes, slot->length));
    printf(
        "[sbb-wrapper] transport poll: channel=%u received length=%u pending\n",
        transport_channel_id,
        slot->length);

#ifdef SBB_WRAPPER_HAS_SBB_REDL
    if (!g_redl_initialized) {
        printf(
            "[sbb-wrapper] redtrn_MessageReceivedNotification: transport=%u deferred because RedL is not initialized\n",
            transport_channel_id);
        return 1;
    }
    if (sbb_wrapper_diag_closed_after_up()) {
        printf(
            "[sbb-wrapper] redtrn_MessageReceivedNotification: transport=%u skipped because connection closed after Up\n",
            transport_channel_id);
        return 1;
    }
    sbb_wrapper_diag_set_phase("transport_poll:redtrn_MessageReceivedNotification");
    redtrn_MessageReceivedNotification(transport_channel_id);
    if (g_disconnect_frame_in_progress && sbb_wrapper_diag_closed_after_up()) {
        puts("[sbb-wrapper] RedL notification stopped safely because DiscReq closed the connection");
    }
    printf(
        "[sbb-wrapper] redtrn_MessageReceivedNotification: transport=%u invoked\n",
        transport_channel_id);
#else
    printf(
        "[sbb-wrapper] redtrn_MessageReceivedNotification: transport=%u SBB RedL not linked\n",
        transport_channel_id);
#endif

    return 1;
}

void sbb_wrapper_transport_poll_all(void)
{
    uint32_t i;
    for (i = 0u; i < SBB_WRAPPER_TRANSPORT_CHANNEL_COUNT; i += 1u) {
        (void)sbb_wrapper_transport_poll_channel(i);
        if (sbb_wrapper_diag_has_fatal() || sbb_wrapper_diag_closed_after_up()) {
            break;
        }
    }
}

uint32_t sbb_wrapper_transport_pending_count(uint32_t transport_channel_id)
{
    SbbWrapperPendingDatagram *slot = pending_slot(transport_channel_id);
    if (slot == 0 || !slot->occupied) {
        return 0u;
    }
    return 1u;
}
