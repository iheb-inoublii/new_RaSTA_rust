#include "sbb_adapter.h"

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
    puts("[sbb-wrapper] redtri_Init: skeleton initialized");
    return radef_kOk;
}

RadefReturnCode redtri_SendMessage(uint32_t transport_channel_id, const uint8_t *message, size_t length)
{
    if (message == 0 && length != 0u) {
        puts("[sbb-wrapper] redtri_SendMessage: invalid null message");
        return radef_kInvalidParameter;
    }

    printf(
        "[sbb-wrapper] redtri_SendMessage: transport=%u length=%zu stubbed-no-send\n",
        transport_channel_id,
        length);
    return radef_kNotImplemented;
}

RadefReturnCode redtri_ReadMessage(
    uint32_t transport_channel_id,
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
        "[sbb-wrapper] redtri_ReadMessage: transport=%u no queued message\n",
        transport_channel_id);
    return radef_kNoMessageReceived;
}
