#include "sbb_diagnostics.h"

static const char *g_role = "unknown";
static const char *g_phase = "startup";
static uint32_t g_connection_id = 0u;
static uint32_t g_sender_id = 0u;
static uint32_t g_receiver_id = 0u;
static int g_debug_no_abort = 0;
static int g_has_fatal = 0;
static int g_has_reached_up = 0;
static int g_closed_after_up = 0;
static radef_RaStaReturnCode g_fatal_reason = radef_kNoError;

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
    if (state == 4) {
        g_has_reached_up = 1;
    } else if (state == 1 && g_has_reached_up) {
        g_closed_after_up = 1;
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
        return "None";
    case 1:
        return "Regular";
    case 2:
        return "Timeout";
    case 3:
        return "ProtocolError";
    default:
        return "Unknown";
    }
}
