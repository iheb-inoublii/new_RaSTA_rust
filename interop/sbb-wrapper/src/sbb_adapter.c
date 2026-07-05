#include "sbb_adapter.h"
#include "udp_transport.h"

#include <stdio.h>

RadefReturnCode sradin_Init(void)
{
    puts("[sbb-wrapper] sradin_Init: skeleton initialized");
    return radef_kOk;
}

RadefReturnCode sradin_OpenRedundancyChannel(uint32_t redundancy_channel_id)
{
    printf("[sbb-wrapper] sradin_OpenRedundancyChannel: channel=%u skeleton open\n", redundancy_channel_id);
    return radef_kOk;
}

RadefReturnCode sradin_CloseRedundancyChannel(uint32_t redundancy_channel_id)
{
    printf("[sbb-wrapper] sradin_CloseRedundancyChannel: channel=%u skeleton close\n", redundancy_channel_id);
    return radef_kOk;
}

RadefReturnCode sradin_SendMessage(uint32_t redundancy_channel_id, const uint8_t *message, size_t length)
{
    if (message == 0 && length != 0u) {
        puts("[sbb-wrapper] sradin_SendMessage: invalid null message");
        return radef_kInvalidParameter;
    }

    printf(
        "[sbb-wrapper] sradin_SendMessage: channel=%u length=%zu stubbed-no-send\n",
        redundancy_channel_id,
        length);
    return radef_kNotImplemented;
}

RadefReturnCode sradin_ReadMessage(
    uint32_t redundancy_channel_id,
    uint8_t *buffer,
    size_t capacity,
    size_t *length)
{
    (void)buffer;
    (void)capacity;

    if (length != 0) {
        *length = 0u;
    }

    printf(
        "[sbb-wrapper] sradin_ReadMessage: channel=%u no queued message\n",
        redundancy_channel_id);
    return radef_kNoMessageReceived;
}

RadefReturnCode redtri_Init(void)
{
    if (!sbb_wrapper_udp_is_initialized()) {
        puts("[sbb-wrapper] redtri_Init: UDP transport is not initialized");
        return radef_kInvalidParameter;
    }

    puts("[sbb-wrapper] redtri_Init: UDP transport ready");
    return radef_kOk;
}

RadefReturnCode redtri_SendMessage(uint32_t transport_channel_id, const uint8_t *message, size_t length)
{
    SbbWrapperUdpResult result;

    if (message == 0 && length != 0u) {
        puts("[sbb-wrapper] redtri_SendMessage: invalid null message");
        return radef_kInvalidParameter;
    }

    result = sbb_wrapper_udp_send(transport_channel_id, message, length);
    if (result == SBB_WRAPPER_UDP_OK) {
        printf(
            "[sbb-wrapper] redtri_SendMessage: transport=%u length=%zu sent\n",
            transport_channel_id,
            length);
        return radef_kOk;
    }

    printf(
        "[sbb-wrapper] redtri_SendMessage: transport=%u length=%zu failed udp_result=%d\n",
        transport_channel_id,
        length,
        result);
    return result == SBB_WRAPPER_UDP_INVALID_CHANNEL || result == SBB_WRAPPER_UDP_INVALID_PARAMETER
        ? radef_kInvalidParameter
        : radef_kNotImplemented;
}

RadefReturnCode redtri_ReadMessage(
    uint32_t transport_channel_id,
    uint8_t *buffer,
    size_t capacity,
    size_t *length)
{
    SbbWrapperUdpResult result;

    if (length != 0) {
        *length = 0u;
    }

    result = sbb_wrapper_udp_receive(transport_channel_id, buffer, capacity, length);
    if (result == SBB_WRAPPER_UDP_OK) {
        printf(
            "[sbb-wrapper] redtri_ReadMessage: transport=%u length=%zu received\n",
            transport_channel_id,
            length == 0 ? 0u : *length);
        return radef_kOk;
    }
    if (result == SBB_WRAPPER_UDP_NO_MESSAGE) {
        printf("[sbb-wrapper] redtri_ReadMessage: transport=%u no message\n", transport_channel_id);
        return radef_kNoMessageReceived;
    }

    printf("[sbb-wrapper] redtri_ReadMessage: transport=%u failed udp_result=%d\n", transport_channel_id, result);
    return result == SBB_WRAPPER_UDP_INVALID_CHANNEL || result == SBB_WRAPPER_UDP_INVALID_PARAMETER
        ? radef_kInvalidParameter
        : radef_kNotImplemented;
}
