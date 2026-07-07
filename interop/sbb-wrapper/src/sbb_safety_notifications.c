#include "udp_transport.h"
#include "sbb_diagnostics.h"

#include <stdio.h>

#ifdef SBB_WRAPPER_HAS_SBB_REDL
#include "rasta_safety_retransmission/srnot_sr_notifications.h"
#else
#include <stdint.h>
typedef int sraty_ConnectionStates;
typedef int sraty_DiscReason;
typedef struct {
    uint16_t send_buffer_used;
    uint16_t send_buffer_free;
    uint16_t receive_buffer_used;
    uint16_t receive_buffer_free;
} sraty_BufferUtilisation;
typedef struct {
    uint32_t ec_safety;
    uint32_t ec_address;
    uint32_t ec_type;
    uint32_t ec_sn;
    uint32_t ec_csn;
} sraty_ConnectionDiagnosticData;
typedef struct {
    uint32_t reserved;
} sraty_RedundancyChannelDiagnosticData;
#endif

void srnot_MessageReceivedNotification(const uint32_t connection_id)
{
    if (sbb_wrapper_udp_trace_enabled()) {
        printf("[sbb-wrapper] srnot_MessageReceivedNotification connection=%u\n", (unsigned int)connection_id);
    }
}

void srnot_ConnectionStateNotification(
    const uint32_t connection_id,
    const sraty_ConnectionStates connection_state,
    const sraty_BufferUtilisation buffer_utilisation,
    const uint16_t opposite_buffer_size,
    const sraty_DiscReason disconnect_reason,
    const uint16_t detailed_disconnect_reason)
{
    if (sbb_wrapper_udp_trace_enabled()) {
        printf(
            "[sbb-wrapper] srnot_ConnectionStateNotification connection=%u state=%d(%s) send_used=%u recv_used=%u opposite_buffer=%u disconnect=%d(%s) detailed=%u\n",
            (unsigned int)connection_id,
            (int)connection_state,
            sbb_wrapper_connection_state_name((int)connection_state),
            (unsigned int)buffer_utilisation.send_buffer_used,
            (unsigned int)buffer_utilisation.receive_buffer_used,
            (unsigned int)opposite_buffer_size,
            (int)disconnect_reason,
            sbb_wrapper_disconnect_reason_name((int)disconnect_reason),
            (unsigned int)detailed_disconnect_reason);
    }
}

void srnot_SrDiagnosticNotification(
    const uint32_t connection_id,
    const sraty_ConnectionDiagnosticData connection_diagnostic_data)
{
    if (sbb_wrapper_udp_trace_enabled()) {
        printf(
            "[sbb-wrapper] srnot_SrDiagnosticNotification connection=%u ec_safety=%u ec_address=%u ec_type=%u ec_sn=%u ec_csn=%u\n",
            (unsigned int)connection_id,
            (unsigned int)connection_diagnostic_data.ec_safety,
            (unsigned int)connection_diagnostic_data.ec_address,
            (unsigned int)connection_diagnostic_data.ec_type,
            (unsigned int)connection_diagnostic_data.ec_sn,
            (unsigned int)connection_diagnostic_data.ec_csn);
    }
}

void srnot_RedDiagnosticNotification(
    const uint32_t connection_id,
    const sraty_RedundancyChannelDiagnosticData redundancy_channel_diagnostic_data)
{
    (void)redundancy_channel_diagnostic_data;
    if (sbb_wrapper_udp_trace_enabled()) {
        printf("[sbb-wrapper] srnot_RedDiagnosticNotification connection=%u\n", (unsigned int)connection_id);
    }
}
