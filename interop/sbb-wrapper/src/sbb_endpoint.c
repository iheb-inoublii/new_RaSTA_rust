#include "sbb_endpoint.h"

#include "ping_pong_payload.h"
#include "sbb_diagnostics.h"

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

uint32_t sbb_endpoint_local_sender_id(const SbbEndpoint *endpoint);
uint32_t sbb_endpoint_remote_receiver_id(const SbbEndpoint *endpoint);

#ifdef SBB_WRAPPER_HAS_SBB_REDL
/*
 * SBB SafRetL matches srapi_OpenConnection arguments against the static
 * sender/receiver tuple and validates incoming frames against the reversed
 * peer tuple. The wrapper therefore keeps role-local configs instead of
 * modifying the external SBB checkout.
 */
static const srcty_SafetyRetransmissionConfiguration k_safretl_active_config = {
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

static const srcty_SafetyRetransmissionConfiguration k_safretl_passive_config = {
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
            .sender_id = SBB_WRAPPER_SAFRETL_RECEIVER_ID,
            .receiver_id = SBB_WRAPPER_SAFRETL_SENDER_ID,
        },
        {
            .connection_id = 1U,
            .sender_id = 3U,
            .receiver_id = 1U,
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
        printf("[sbb-wrapper] %s result=%d(%s)\n", label, result, sbb_wrapper_rasta_return_code_name(result));
    }
}

static const char *sbb_endpoint_role_name(const SbbEndpoint *endpoint)
{
    return endpoint->role == SBB_ENDPOINT_ROLE_ACTIVE ? "active" : "passive";
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
    return sbb_wrapper_connection_state_name(state);
}

radef_RaStaReturnCode sbb_endpoint_init(SbbEndpoint *endpoint)
{
#ifdef SBB_WRAPPER_HAS_SBB_REDL
    radef_RaStaReturnCode result;

    sbb_wrapper_diag_set_phase("sbb_endpoint_init");
    sbb_wrapper_diag_set_context(
        sbb_endpoint_role_name(endpoint),
        endpoint->connection_id,
        sbb_endpoint_local_sender_id(endpoint),
        sbb_endpoint_remote_receiver_id(endpoint));
    sradin_Init();
    result = srapi_Init(endpoint->role == SBB_ENDPOINT_ROLE_ACTIVE ? &k_safretl_active_config : &k_safretl_passive_config);
    endpoint->initialized = (result == radef_kNoError);
    printf(
        "[sbb-wrapper] SafRetL role=%s connection_id=%u local_sender_id=0x%02x remote_receiver_id=0x%02x network_id=%u\n",
        sbb_endpoint_role_name(endpoint),
        (unsigned int)endpoint->connection_id,
        (unsigned int)sbb_endpoint_local_sender_id(endpoint),
        (unsigned int)sbb_endpoint_remote_receiver_id(endpoint),
        (unsigned int)SBB_WRAPPER_SAFRETL_NETWORK_ID);
    trace_result(endpoint, "srapi_Init", result);
    sbb_wrapper_diag_set_context(
        sbb_endpoint_role_name(endpoint),
        endpoint->connection_id,
        sbb_endpoint_local_sender_id(endpoint),
        sbb_endpoint_remote_receiver_id(endpoint));
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

    sbb_wrapper_diag_set_phase("sbb_endpoint_open");
    sbb_wrapper_diag_set_context(
        sbb_endpoint_role_name(endpoint),
        endpoint->connection_id,
        sbb_endpoint_local_sender_id(endpoint),
        sbb_endpoint_remote_receiver_id(endpoint));
    printf(
        "[sbb-wrapper] SafRetL open: role=%s call_srapi_OpenConnection=true sender_id=0x%02x receiver_id=0x%02x network_id=%u\n",
        sbb_endpoint_role_name(endpoint),
        (unsigned int)sbb_endpoint_local_sender_id(endpoint),
        (unsigned int)sbb_endpoint_remote_receiver_id(endpoint),
        (unsigned int)SBB_WRAPPER_SAFRETL_NETWORK_ID);

    result = srapi_OpenConnection(
        sbb_endpoint_local_sender_id(endpoint),
        sbb_endpoint_remote_receiver_id(endpoint),
        SBB_WRAPPER_SAFRETL_NETWORK_ID,
        &endpoint->connection_id);
    endpoint->open_requested = (result == radef_kNoError);
    sbb_wrapper_diag_set_context(
        sbb_endpoint_role_name(endpoint),
        endpoint->connection_id,
        sbb_endpoint_local_sender_id(endpoint),
        sbb_endpoint_remote_receiver_id(endpoint));
    printf(
        "[sbb-wrapper] srapi_OpenConnection: role=%s result=%d(%s) returned_connection_id=%u\n",
        sbb_endpoint_role_name(endpoint),
        result,
        sbb_wrapper_rasta_return_code_name(result),
        (unsigned int)endpoint->connection_id);
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
    radef_RaStaReturnCode result;

    endpoint->poll_count += 1U;
    sbb_wrapper_diag_set_phase("sbb_endpoint_poll:transport_poll_all");
    sbb_wrapper_transport_poll_all();

    if (sbb_wrapper_diag_has_fatal()) {
        return sbb_wrapper_diag_fatal_reason();
    }

    sbb_wrapper_diag_set_phase("sbb_endpoint_poll:srapi_CheckTimings");
    result = srapi_CheckTimings();
    if (endpoint->trace) {
        printf("[sbb-wrapper] srapi_CheckTimings result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
    }
    if (result != radef_kNoError) {
        return result;
    }

    sbb_wrapper_diag_set_phase("sbb_endpoint_poll:srapi_GetConnectionState");
    result = srapi_GetConnectionState(
        endpoint->connection_id,
        &state,
        &buffer_utilisation,
        &opposite_buffer_size);
    if (endpoint->trace) {
        printf(
            "[sbb-wrapper] srapi_GetConnectionState: connection=%u result=%d state=%s send_used=%u recv_used=%u opposite_buffer=%u\n",
            (unsigned int)endpoint->connection_id,
            result,
            result == radef_kNoError ? sbb_endpoint_state_name((int)state) : "Unavailable",
            (unsigned int)buffer_utilisation.send_buffer_used,
            (unsigned int)buffer_utilisation.receive_buffer_used,
            (unsigned int)opposite_buffer_size);
    }
    if (result == radef_kNoError) {
        int state_value = (int)state;
        if (endpoint->trace && state_value != endpoint->last_state) {
            printf(
                "[sbb-wrapper] connection %u state transition %s -> %s send_used=%u recv_used=%u opposite_buffer=%u\n",
                (unsigned int)endpoint->connection_id,
                sbb_endpoint_state_name(endpoint->last_state),
                sbb_endpoint_state_name(state_value),
                (unsigned int)buffer_utilisation.send_buffer_used,
                (unsigned int)buffer_utilisation.receive_buffer_used,
                (unsigned int)opposite_buffer_size);
        }
        endpoint->last_state = state_value;
        if (state_value == 4) {
            endpoint->has_reached_up = 1;
        } else if (state_value == 1 && endpoint->has_reached_up) {
            endpoint->closed_after_up = 1;
            if (endpoint->trace) {
                puts("[sbb-wrapper] graceful close observed after Up; stopping SafRetL/RedL polling");
            }
        }
    }

    if (sbb_wrapper_diag_has_fatal()) {
        return sbb_wrapper_diag_fatal_reason();
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

    sbb_wrapper_diag_set_phase("sbb_endpoint_send_ping:srapi_SendData");
    result = srapi_SendData(endpoint->connection_id, (uint16_t)payload_length, payload);
    if (endpoint->trace) {
        printf(
            "[sbb-wrapper] srapi_SendData Ping(%u) result=%d(%s)\n",
            (unsigned int)counter,
            result,
            sbb_wrapper_rasta_return_code_name(result));
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
    radef_RaStaReturnCode result;

    sbb_wrapper_diag_set_phase("sbb_endpoint_read:srapi_ReadData");
    result = srapi_ReadData(
        endpoint->connection_id,
        (uint16_t)sizeof(payload),
        &payload_length,
        payload);

    if (endpoint->trace) {
        printf(
            "[sbb-wrapper] srapi_ReadData: connection=%u result=%d length=%u\n",
            (unsigned int)endpoint->connection_id,
            result,
            (unsigned int)payload_length);
    }

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
    }

    if (sbb_wrapper_diag_has_fatal()) {
        return sbb_wrapper_diag_fatal_reason();
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
    if (endpoint->closed_after_up) {
        if (endpoint->trace) {
            puts("[sbb-wrapper] SafRetL close skipped because connection already closed after Up");
        }
        return radef_kNoError;
    }

    sbb_wrapper_diag_set_phase("sbb_endpoint_close:srapi_CloseConnection");
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

int sbb_endpoint_is_closed_after_up(const SbbEndpoint *endpoint)
{
    return endpoint->closed_after_up;
}

uint32_t sbb_endpoint_local_sender_id(const SbbEndpoint *endpoint)
{
    return endpoint->role == SBB_ENDPOINT_ROLE_ACTIVE ? SBB_WRAPPER_SAFRETL_SENDER_ID : SBB_WRAPPER_SAFRETL_RECEIVER_ID;
}

uint32_t sbb_endpoint_remote_receiver_id(const SbbEndpoint *endpoint)
{
    return endpoint->role == SBB_ENDPOINT_ROLE_ACTIVE ? SBB_WRAPPER_SAFRETL_RECEIVER_ID : SBB_WRAPPER_SAFRETL_SENDER_ID;
}
