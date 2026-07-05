#include "sbb_adapter.h"
#include "udp_transport.h"

#include <stdio.h>

#ifdef SBB_WRAPPER_HAS_SBB_REDL
#include "rasta_redundancy/redcty_red_config_types.h"
#include "rasta_redundancy/redint_red_interface.h"

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

void sradin_Init(void)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result = redint_Init(&g_redl_config);
    printf("[sbb-wrapper] sradin_Init: redint_Init result=%u\n", result);
#else
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
    SbbWrapperUdpResult result;
    size_t received_length = 0u;

    if (message_size != 0) {
        *message_size = 0u;
    }

    result = sbb_wrapper_udp_receive(transport_channel_id, message_buffer, buffer_size, &received_length);
    if (result == SBB_WRAPPER_UDP_OK) {
        if (message_size != 0) {
            *message_size = (uint16_t)received_length;
        }
        printf(
            "[sbb-wrapper] redtri_ReadMessage: transport=%u length=%u received\n",
            transport_channel_id,
            message_size == 0 ? 0u : *message_size);
        return radef_kNoError;
    }
    if (result == SBB_WRAPPER_UDP_NO_MESSAGE) {
        printf("[sbb-wrapper] redtri_ReadMessage: transport=%u no message\n", transport_channel_id);
        return radef_kNoMessageReceived;
    }

    printf("[sbb-wrapper] redtri_ReadMessage: transport=%u failed udp_result=%d\n", transport_channel_id, result);
    return result == SBB_WRAPPER_UDP_INVALID_CHANNEL || result == SBB_WRAPPER_UDP_INVALID_PARAMETER
        ? radef_kInvalidParameter
        : radef_kInternalError;
}
