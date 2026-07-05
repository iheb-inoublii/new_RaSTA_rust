#include "sbb_endpoint.h"

#include "ping_pong_payload.h"

#include <stddef.h>
#include <stdio.h>
#include <string.h>

#ifdef SBB_WRAPPER_HAS_SBB_REDL
#include "rasta_safety_retransmission/srapi_sr_api.h"
#include "rasta_safety_retransmission/sraty_sr_api_types.h"
#include "rasta_safety_retransmission/srcty_sr_config_types.h"
#endif

#ifndef RADEF_MAX_SR_LAYER_PAYLOAD_DATA_SIZE
#define RADEF_MAX_SR_LAYER_PAYLOAD_DATA_SIZE 1055U
#endif

#ifdef SBB_WRAPPER_HAS_SBB_REDL
static const srcty_SafetyRetransmissionConfiguration k_safretl_config = {
    .rasta_network_id = SBB_WRAPPER_SAFRETL_NETWORK_ID,
    .t_max = 750U,
    .t_h = 300U,
    .safety_code_type = srcty_kSafetyCodeTypeLowerMd4,
    .m_w_a = 10U,
    .n_send_max = 20U,
    .n_max_packet = 1U,
    .n_diag_window = 5000U,
    .number_of_connections = 2U,
    {
        {
            .connection_id = 0U,
            .sender_id = SBB_WRAPPER_SAFRETL_SENDER_ID,
            .receiver_id = SBB_WRAPPER_SAFRETL_RECEIVER_ID,
        },
        {
            .connection_id = 1U,
            .sender_id = 1U,
            .receiver_id = 3U,
        },
    },
    {
        .init_a = 0x67452301U,
        .init_b = 0xEFCDAB89U,
        .init_c = 0x98BADCFEU,
        .init_d = 0x10325476U,
    },
    {
        150U,
        300U,
        450U,
        600U,
    },
};
#endif

static void trace_result(const SbbEndpoint *endpoint, const char *label, radef_RaStaReturnCode result)
{
    if (endpoint->trace) {
        printf("[sbb-wrapper] %s result=%d\n", label, result);
    }
}

void sbb_endpoint_configure(SbbEndpoint *endpoint, SbbEndpointRole role, int trace)
{
    memset(endpoint, 0, sizeof(*endpoint));
    endpoint->role = role;
    endpoint->trace = trace;
    endpoint->connection_id = SBB_WRAPPER_SAFRETL_CONNECTION_ID;
    endpoint->last_state = 0;
}

const char *sbb_endpoint_state_name(int state)
{
    switch (state) {
    case 0:
        return "NotInitialized";
    case 1:
        return "Closed";
    case 2:
        return "Down";
    case 3:
        return "Start";
    case 4:
        return "Up";
    case 5:
        return "RetransRequest";
    case 6:
        return "RetransRunning";
    default:
        return "Unknown";
    }
}

radef_RaStaReturnCode sbb_endpoint_init(SbbEndpoint *endpoint)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result;

    sradin_Init();
    result = srapi_Init(&k_safretl_config);
    endpoint->initialized = (result == radef_kNoError);
    trace_result(endpoint, "srapi_Init", result);
    return result;
#else
    (void)endpoint;
    puts("[sbb-wrapper] SafRetL smoke requires SBB_ROOT; no SBB SafRetL library is linked");
    return radef_kNotInitialized;
#endif
}

radef_RaStaReturnCode sbb_endpoint_open(SbbEndpoint *endpoint)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result;

    if (endpoint->role == SBB_ENDPOINT_ROLE_PASSIVE) {
        endpoint->open_requested = 0;
        return radef_kNoError;
    }

    result = srapi_OpenConnection(
        SBB_WRAPPER_SAFRETL_SENDER_ID,
        SBB_WRAPPER_SAFRETL_RECEIVER_ID,
        SBB_WRAPPER_SAFRETL_NETWORK_ID,
        &endpoint->connection_id);
    endpoint->open_requested = (result == radef_kNoError);
    trace_result(endpoint, "srapi_OpenConnection", result);
    return result;
#else
    (void)endpoint;
    return radef_kNotInitialized;
#endif
}

radef_RaStaReturnCode sbb_endpoint_poll(SbbEndpoint *endpoint)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    sraty_ConnectionStates state = sraty_kConnectionNotInitialized;
    sraty_BufferUtilisation buffer_utilisation = {0};
    uint16_t opposite_buffer_size = 0U;
    radef_RaStaReturnCode result = srapi_CheckTimings();

    endpoint->poll_count += 1U;
    if (result != radef_kNoError) {
        trace_result(endpoint, "srapi_CheckTimings", result);
        return result;
    }

    result = srapi_GetConnectionState(
        endpoint->connection_id,
        &state,
        &buffer_utilisation,
        &opposite_buffer_size);
    if (result == radef_kNoError) {
        int state_value = (int)state;
        if (endpoint->trace && (state_value != endpoint->last_state || (endpoint->poll_count % 100U) == 0U)) {
            printf(
                "[sbb-wrapper] connection %u state=%s send_used=%u recv_used=%u opposite_buffer=%u\n",
                (unsigned int)endpoint->connection_id,
                sbb_endpoint_state_name(state_value),
                (unsigned int)buffer_utilisation.send_buffer_used,
                (unsigned int)buffer_utilisation.receive_buffer_used,
                (unsigned int)opposite_buffer_size);
        }
        endpoint->last_state = state_value;
    }

    return result;
#else
    (void)endpoint;
    return radef_kNotInitialized;
#endif
}

radef_RaStaReturnCode sbb_endpoint_send_ping(SbbEndpoint *endpoint, uint32_t counter)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    uint8_t payload[SBB_WRAPPER_PING_PONG_PAYLOAD_LEN] = {0};
    size_t payload_length = 0U;
    radef_RaStaReturnCode result;

    if (!sbb_endpoint_is_up(endpoint)) {
        return radef_kInvalidOperationInCurrentState;
    }

    if (sbb_wrapper_encode_ping(counter, payload, sizeof(payload), &payload_length) != SBB_WRAPPER_PAYLOAD_OK) {
        return radef_kInternalError;
    }

    result = srapi_SendData(endpoint->connection_id, (uint16_t)payload_length, payload);
    if (endpoint->trace) {
        printf("[sbb-wrapper] srapi_SendData Ping(%u) result=%d\n", (unsigned int)counter, result);
    }
    return result;
#else
    (void)endpoint;
    (void)counter;
    return radef_kNotInitialized;
#endif
}

radef_RaStaReturnCode sbb_endpoint_read(SbbEndpoint *endpoint)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    uint8_t payload[RADEF_MAX_SR_LAYER_PAYLOAD_DATA_SIZE] = {0};
    uint16_t payload_length = 0U;
    radef_RaStaReturnCode result = srapi_ReadData(
        endpoint->connection_id,
        (uint16_t)sizeof(payload),
        &payload_length,
        payload);

    if (result == radef_kNoError) {
        SbbWrapperPayloadKind kind;
        uint32_t counter = 0U;
        SbbWrapperPayloadResult decode_result = sbb_wrapper_decode_ping_pong(payload, payload_length, &kind, &counter);
        if (decode_result == SBB_WRAPPER_PAYLOAD_OK) {
            printf(
                "[sbb-wrapper] received %s(%u)\n",
                kind == SBB_WRAPPER_PAYLOAD_KIND_PING ? "Ping" : "Pong",
                (unsigned int)counter);
        } else if (endpoint->trace) {
            printf(
                "[sbb-wrapper] received SafRetL payload length=%u decode_result=%d\n",
                (unsigned int)payload_length,
                decode_result);
        }
    } else if (endpoint->trace && result != radef_kNoMessageReceived) {
        printf("[sbb-wrapper] srapi_ReadData result=%d\n", result);
    }

    return result;
#else
    (void)endpoint;
    return radef_kNotInitialized;
#endif
}

radef_RaStaReturnCode sbb_endpoint_close(SbbEndpoint *endpoint)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result;

    if (!endpoint->initialized) {
        return radef_kNotInitialized;
    }

    result = srapi_CloseConnection(endpoint->connection_id, 0U);
    trace_result(endpoint, "srapi_CloseConnection", result);
    return result;
#else
    (void)endpoint;
    return radef_kNotInitialized;
#endif
}

int sbb_endpoint_is_up(const SbbEndpoint *endpoint)
{
    return endpoint->last_state == 4;
}
