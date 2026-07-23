#include "sbb_diagnostics.h"
#include "sbb_timeout_instrumentation.h"

#include <stdio.h>

static const char *g_role = "unknown";
static const char *g_phase = "startup";
static uint32_t g_connection_id = 0u;
static uint32_t g_sender_id = 0u;
static uint32_t g_receiver_id = 0u;
static int g_debug_no_abort = 0;
static int g_has_fatal = 0;
static int g_has_reached_up = 0;
static int g_closed_after_up = 0;
static uint32_t g_heartbeat_count = 0u;
static int g_application_complete = 0;
static radef_RaStaReturnCode g_fatal_reason = radef_kNoError;
static int g_last_connection_state = 0;
static int g_state_before_disconnect = 0;
static int g_disconnect_reason = -1;
static uint16_t g_detailed_disconnect_reason = 0u;
static uint32_t g_last_successful_ping = 0u;
static uint16_t g_last_send_used = 0u;
static uint16_t g_max_send_used = 0u;
static uint16_t g_last_recv_used = 0u;
static uint16_t g_opposite_buffer = 0u;
static radef_RaStaReturnCode g_last_check_timings_result = radef_kNoError;
static radef_RaStaReturnCode g_last_read_data_result = radef_kNoMessageReceived;
static radef_RaStaReturnCode g_last_send_data_result = radef_kNoError;
static uint32_t g_ec_safety = 0u;
static uint32_t g_ec_address = 0u;
static uint32_t g_ec_type = 0u;
static uint32_t g_ec_sn = 0u;
static uint32_t g_ec_csn = 0u;
static uint32_t g_timeout_branch_counts[SBB_WRAPPER_TIMEOUT_BRANCH_COUNT] = {0u};
static int g_last_timeout_branch = -1;

void sbb_wrapper_diag_set_context(const char *role, uint32_t connection_id, uint32_t sender_id, uint32_t receiver_id)
{
    g_role = role == 0 ? "unknown" : role;
    g_connection_id = connection_id;
    g_sender_id = sender_id;
    g_receiver_id = receiver_id;
}

void sbb_wrapper_diag_set_phase(const char *phase)
{
    g_phase = phase == 0 ? "unknown" : phase;
}

void sbb_wrapper_diag_set_debug_no_abort(int enabled)
{
    g_debug_no_abort = enabled != 0;
}

int sbb_wrapper_diag_debug_no_abort(void)
{
    return g_debug_no_abort;
}

void sbb_wrapper_diag_record_fatal(radef_RaStaReturnCode reason)
{
    g_has_fatal = 1;
    g_fatal_reason = reason;
}

int sbb_wrapper_diag_has_fatal(void)
{
    return g_has_fatal;
}

radef_RaStaReturnCode sbb_wrapper_diag_fatal_reason(void)
{
    return g_fatal_reason;
}

void sbb_wrapper_diag_observe_connection_state(int state)
{
    if (state != g_last_connection_state) {
        g_state_before_disconnect = g_last_connection_state;
    }
    g_last_connection_state = state;
    if (state == 4) {
        g_has_reached_up = 1;
    } else if (state == 1 && g_has_reached_up) {
        g_closed_after_up = 1;
    }
}

void sbb_wrapper_diag_observe_sr_type(uint16_t sr_type)
{
    if (sr_type == 6220u) {
        g_heartbeat_count += 1u;
    }
}

int sbb_wrapper_diag_has_reached_up(void)
{
    return g_has_reached_up;
}

int sbb_wrapper_diag_closed_after_up(void)
{
    return g_closed_after_up;
}

uint32_t sbb_wrapper_diag_heartbeat_count(void)
{
    return g_heartbeat_count;
}

void sbb_wrapper_diag_mark_application_complete(void)
{
    g_application_complete = 1;
}

void sbb_wrapper_diag_observe_connection_snapshot(int state, uint16_t send_used, uint16_t recv_used, uint16_t opposite_buffer)
{
    sbb_wrapper_diag_observe_connection_state(state);
    g_last_send_used = send_used;
    if (send_used > g_max_send_used) {
        g_max_send_used = send_used;
    }
    g_last_recv_used = recv_used;
    g_opposite_buffer = opposite_buffer;
}

void sbb_wrapper_diag_observe_connection_notification(
    int state,
    uint16_t send_used,
    uint16_t recv_used,
    uint16_t opposite_buffer,
    int disconnect_reason,
    uint16_t detailed_disconnect_reason)
{
    sbb_wrapper_diag_observe_connection_snapshot(state, send_used, recv_used, opposite_buffer);
    if (state == 1) {
        g_disconnect_reason = disconnect_reason;
        g_detailed_disconnect_reason = detailed_disconnect_reason;
    }
}

void sbb_wrapper_diag_observe_check_timings_result(radef_RaStaReturnCode result)
{
    g_last_check_timings_result = result;
}

void sbb_wrapper_diag_observe_read_data_result(radef_RaStaReturnCode result)
{
    g_last_read_data_result = result;
}

void sbb_wrapper_diag_observe_send_data_result(radef_RaStaReturnCode result)
{
    g_last_send_data_result = result;
}

void sbb_wrapper_diag_observe_protocol_counters(
    uint32_t ec_safety,
    uint32_t ec_address,
    uint32_t ec_type,
    uint32_t ec_sn,
    uint32_t ec_csn)
{
    g_ec_safety = ec_safety;
    g_ec_address = ec_address;
    g_ec_type = ec_type;
    g_ec_sn = ec_sn;
    g_ec_csn = ec_csn;
}

void sbb_wrapper_diag_observe_successful_ping(uint32_t counter)
{
    g_last_successful_ping = counter;
}

void sbb_wrapper_diag_note_timeout_branch(uint32_t connection_id, uint32_t branch)
{
    if (connection_id == g_connection_id && branch < SBB_WRAPPER_TIMEOUT_BRANCH_COUNT) {
        g_timeout_branch_counts[branch] += 1u;
        g_last_timeout_branch = (int)branch;
    }
}

int sbb_wrapper_diag_application_complete(void)
{
    return g_application_complete;
}

const char *sbb_wrapper_diag_role(void)
{
    return g_role;
}

uint32_t sbb_wrapper_diag_connection_id(void)
{
    return g_connection_id;
}

uint32_t sbb_wrapper_diag_sender_id(void)
{
    return g_sender_id;
}

uint32_t sbb_wrapper_diag_receiver_id(void)
{
    return g_receiver_id;
}

const char *sbb_wrapper_diag_phase(void)
{
    return g_phase;
}

const char *sbb_wrapper_rasta_return_code_name(radef_RaStaReturnCode code)
{
    switch (code) {
    case radef_kNoError:
        return "NoError";
    case radef_kNoMessageReceived:
        return "NoMessageReceived";
    case radef_kNoMessageToSend:
        return "NoMessageToSend";
    case radef_kNotInitialized:
        return "NotInitialized";
    case radef_kAlreadyInitialized:
        return "AlreadyInitialized";
    case radef_kInvalidConfiguration:
        return "InvalidConfiguration";
    case radef_kInvalidParameter:
        return "InvalidParameter";
    case radef_kInvalidMessageType:
        return "InvalidMessageType";
    case radef_kInvalidMessageSize:
        return "InvalidMessageSize";
    case radef_kInvalidBufferSize:
        return "InvalidBufferSize";
    case radef_kInvalidMessageCrc:
        return "InvalidMessageCrc";
    case radef_kInvalidMessageMd4:
        return "InvalidMessageMd4";
    case radef_kReceiveBufferFull:
        return "ReceiveBufferFull";
    case radef_kDeferQueueEmpty:
        return "DeferQueueEmpty";
    case radef_kSendBufferFull:
        return "SendBufferFull";
    case radef_kInvalidSequenceNumber:
        return "InvalidSequenceNumber";
    case radef_kInternalError:
        return "InternalError";
    case radef_kInvalidOperationInCurrentState:
        return "InvalidOperationInCurrentState";
    default:
        return "Unknown";
    }
}

const char *sbb_wrapper_connection_state_name(int state)
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

const char *sbb_wrapper_disconnect_reason_name(int reason)
{
    switch (reason) {
    case 0:
        return "sraty_kDiscReasonUserRequest";
    case 1:
        return "sraty_kDiscReasonNotInUse";
    case 2:
        return "sraty_kDiscReasonUnexpectedMessage";
    case 3:
        return "sraty_kDiscReasonSequenceNumberError";
    case 4:
        return "sraty_kDiscReasonTimeout";
    case 5:
        return "sraty_kDiscReasonServiceNotAllowed";
    case 6:
        return "sraty_kDiscReasonProtocolVersionError";
    case 7:
        return "sraty_kDiscReasonRetransmissionFailed";
    case 8:
        return "sraty_kDiscReasonProtocolSequenceError";
    default:
        return "Unknown";
    }
}

static const char *timeout_branch_name(int branch)
{
    switch (branch) {
    case SBB_WRAPPER_TIMEOUT_START_EVENT:
        return "start_event_timeout";
    case SBB_WRAPPER_TIMEOUT_UP_EVENT:
        return "up_event_timeout";
    case SBB_WRAPPER_TIMEOUT_RETRANS_REQUEST_EVENT:
        return "retrans_request_event_timeout";
    case SBB_WRAPPER_TIMEOUT_RETRANS_RUNNING_EVENT:
        return "retrans_running_event_timeout";
    case SBB_WRAPPER_TIMEOUT_RECEIVED_MESSAGE_TIMELINESS:
        return "received_message_timeliness";
    default:
        return "not_observed";
    }
}

void sbb_wrapper_diag_print_final(
    uint32_t requested_rounds,
    uint32_t received_pings,
    uint32_t sent_pongs,
    uint32_t malformed_or_mismatched,
    int success)
{
    const char *disconnect_name = g_disconnect_reason < 0 ? "not_observed" : sbb_wrapper_disconnect_reason_name(g_disconnect_reason);
    printf(
        "[sbb-wrapper] final diagnostics: requested_rounds=%u received_pings=%u sent_pongs=%u malformed_or_mismatched=%u success=%s\n",
        (unsigned int)requested_rounds,
        (unsigned int)received_pings,
        (unsigned int)sent_pongs,
        (unsigned int)malformed_or_mismatched,
        success ? "true" : "false");
    printf(
        "[sbb-wrapper] disconnect: symbolic=%s numeric=%d detailed=%u last_connection_state=%s(%d) state_before_disconnect=%s(%d) last_successful_ping_counter=%u\n",
        disconnect_name,
        g_disconnect_reason,
        (unsigned int)g_detailed_disconnect_reason,
        sbb_wrapper_connection_state_name(g_last_connection_state),
        g_last_connection_state,
        sbb_wrapper_connection_state_name(g_state_before_disconnect),
        g_state_before_disconnect,
        (unsigned int)g_last_successful_ping);
    printf(
        "[sbb-wrapper] buffers_and_calls: last_send_used=%u max_send_used=%u last_recv_used=%u opposite_buffer=%u last_CheckTimings=%d(%s) last_ReadData=%d(%s) last_SendData=%d(%s)\n",
        (unsigned int)g_last_send_used,
        (unsigned int)g_max_send_used,
        (unsigned int)g_last_recv_used,
        (unsigned int)g_opposite_buffer,
        g_last_check_timings_result,
        sbb_wrapper_rasta_return_code_name(g_last_check_timings_result),
        g_last_read_data_result,
        sbb_wrapper_rasta_return_code_name(g_last_read_data_result),
        g_last_send_data_result,
        sbb_wrapper_rasta_return_code_name(g_last_send_data_result));
    printf(
        "[sbb-wrapper] protocol_counters: ec_safety=%u ec_address=%u ec_type=%u ec_sn=%u ec_csn=%u\n",
        (unsigned int)g_ec_safety,
        (unsigned int)g_ec_address,
        (unsigned int)g_ec_type,
        (unsigned int)g_ec_sn,
        (unsigned int)g_ec_csn);
    printf(
        "[sbb-wrapper] timeout_reason4_branches: last=%s start_event=%u up_event=%u retrans_request_event=%u retrans_running_event=%u received_message_timeliness=%u\n",
        timeout_branch_name(g_last_timeout_branch),
        (unsigned int)g_timeout_branch_counts[SBB_WRAPPER_TIMEOUT_START_EVENT],
        (unsigned int)g_timeout_branch_counts[SBB_WRAPPER_TIMEOUT_UP_EVENT],
        (unsigned int)g_timeout_branch_counts[SBB_WRAPPER_TIMEOUT_RETRANS_REQUEST_EVENT],
        (unsigned int)g_timeout_branch_counts[SBB_WRAPPER_TIMEOUT_RETRANS_RUNNING_EVENT],
        (unsigned int)g_timeout_branch_counts[SBB_WRAPPER_TIMEOUT_RECEIVED_MESSAGE_TIMELINESS]);
}
