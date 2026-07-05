#ifndef SBB_WRAPPER_SBB_ADAPTER_H
#define SBB_WRAPPER_SBB_ADAPTER_H

#include <stddef.h>
#include <stdint.h>

typedef enum RadefReturnCode {
    radef_kOk = 0,
    radef_kNoMessageReceived = 1,
    radef_kInvalidParameter = 2,
    radef_kNotImplemented = 3
} RadefReturnCode;

RadefReturnCode sradin_Init(void);
RadefReturnCode sradin_OpenRedundancyChannel(uint32_t redundancy_channel_id);
RadefReturnCode sradin_CloseRedundancyChannel(uint32_t redundancy_channel_id);
RadefReturnCode sradin_SendMessage(uint32_t redundancy_channel_id, const uint8_t *message, size_t length);
RadefReturnCode sradin_ReadMessage(
    uint32_t redundancy_channel_id,
    uint8_t *buffer,
    size_t capacity,
    size_t *length);

RadefReturnCode redtri_Init(void);
RadefReturnCode redtri_SendMessage(uint32_t transport_channel_id, const uint8_t *message, size_t length);
RadefReturnCode redtri_ReadMessage(
    uint32_t transport_channel_id,
    uint8_t *buffer,
    size_t capacity,
    size_t *length);

#endif
