#include "sbb_adapter.h"
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

typedef struct SbbWrapperPendingDatagram {
    int occupied;
    uint16_t length;
    uint8_t bytes[RADEF_MAX_RED_LAYER_PDU_MESSAGE_SIZE];
} SbbWrapperPendingDatagram;

static SbbWrapperPendingDatagram g_pending[SBB_WRAPPER_TRANSPORT_CHANNEL_COUNT];
static int g_redl_initialized = 0;

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

void sradin_Init(void)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result = redint_Init(&g_redl_config);
    g_redl_initialized = (result == radef_kNoError || result == radef_kAlreadyInitialized);
    printf("[sbb-wrapper] sradin_Init: redint_Init result=%u\n", result);
#else
    g_redl_initialized = 0;
    puts("[sbb-wrapper] sradin_Init: SBB RedL not linked");
#endif
}

void sradin_OpenRedundancyChannel(uint32_t redundancy_channel_id)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result = redint_OpenRedundancyChannel(redundancy_channel_id);
    printf("[sbb-wrapper] sradin_OpenRedundancyChannel: channel=%u result=%u\n", redundancy_channel_id, result);
#else
    printf("[sbb-wrapper] sradin_OpenRedundancyChannel: channel=%u SBB RedL not linked\n", redundancy_channel_id);
#endif
}

void sradin_CloseRedundancyChannel(uint32_t redundancy_channel_id)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result = redint_CloseRedundancyChannel(redundancy_channel_id);
    printf("[sbb-wrapper] sradin_CloseRedundancyChannel: channel=%u result=%u\n", redundancy_channel_id, result);
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
        radef_RaStaReturnCode result = redint_SendMessage(redundancy_channel_id, message_size, message_data);
        printf(
            "[sbb-wrapper] sradin_SendMessage: channel=%u length=%u redint_SendMessage result=%u\n",
            redundancy_channel_id,
            message_size,
            result);
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
        radef_RaStaReturnCode timing_result = redint_CheckTimings();
        radef_RaStaReturnCode read_result = redint_ReadMessage(redundancy_channel_id, buffer_size, message_size, message_buffer);
        printf(
            "[sbb-wrapper] sradin_ReadMessage: channel=%u timing_result=%u read_result=%u length=%u\n",
            redundancy_channel_id,
            timing_result,
            read_result,
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
    redtrn_MessageReceivedNotification(transport_channel_id);
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
