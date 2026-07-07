#ifndef SBB_WRAPPER_SBB_ADAPTER_H
#define SBB_WRAPPER_SBB_ADAPTER_H

#include <stddef.h>
#include <stdint.h>

#ifdef SBB_WRAPPER_HAS_SBB_REDL
#include "rasta_common/radef_rasta_definitions.h"
#include "rasta_redundancy/redtri_transport_interface.h"
#include "rasta_safety_retransmission/sradin_sr_adapter_interface.h"
#else
#define RADEF_SR_LAYER_MESSAGE_HEADER_SIZE 28U
#define RADEF_MAX_SR_LAYER_PAYLOAD_DATA_SIZE 1055U
#define RADEF_MAX_SR_LAYER_PDU_MESSAGE_SIZE 1101U
#define RADEF_RED_LAYER_MESSAGE_HEADER_SIZE 8U
#define RADEF_MIN_RED_LAYER_PDU_MESSAGE_SIZE 36U
#define RADEF_MAX_RED_LAYER_PDU_MESSAGE_SIZE 1109U

typedef enum {
    radef_kNoError = 0,
    radef_kNoMessageReceived = 1,
    radef_kNoMessageToSend = 2,
    radef_kNotInitialized = 3,
    radef_kAlreadyInitialized = 4,
    radef_kInvalidConfiguration = 5,
    radef_kInvalidParameter = 6,
    radef_kInvalidMessageType = 7,
    radef_kInvalidMessageSize = 8,
    radef_kInvalidBufferSize = 9,
    radef_kInvalidMessageCrc = 10,
    radef_kInvalidMessageMd4 = 11,
    radef_kReceiveBufferFull = 12,
    radef_kDeferQueueEmpty = 13,
    radef_kSendBufferFull = 14,
    radef_kInvalidSequenceNumber = 15,
    radef_kInternalError = 16,
    radef_kInvalidOperationInCurrentState = 17
} radef_RaStaReturnCode;

typedef struct {
    uint32_t n_diagnosis;
    uint32_t n_missed;
    uint32_t t_drift;
    uint32_t t_drift2;
} radef_TransportChannelDiagnosticData;

void sradin_Init(void);
void sradin_OpenRedundancyChannel(uint32_t redundancy_channel_id);
void sradin_CloseRedundancyChannel(uint32_t redundancy_channel_id);
void sradin_SendMessage(uint32_t redundancy_channel_id, uint16_t message_size, const uint8_t *message_data);
radef_RaStaReturnCode sradin_ReadMessage(
    uint32_t redundancy_channel_id,
    uint16_t buffer_size,
    uint16_t *message_size,
    uint8_t *message_buffer);

void redtri_Init(void);
void redtri_SendMessage(uint32_t transport_channel_id, uint16_t message_size, const uint8_t *message_data);
radef_RaStaReturnCode redtri_ReadMessage(
    uint32_t transport_channel_id,
    uint16_t buffer_size,
    uint16_t *message_size,
    uint8_t *message_buffer);
#endif

void sbb_wrapper_transport_poll_all(void);
int sbb_wrapper_transport_poll_channel(uint32_t transport_channel_id);
uint32_t sbb_wrapper_transport_pending_count(uint32_t transport_channel_id);
int sbb_wrapper_redl_begin_message_notification(uint32_t redundancy_channel_id);
void sbb_wrapper_redl_end_message_notification(uint32_t redundancy_channel_id);

#endif
