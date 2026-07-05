#include "udp_transport.h"

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
    uint32_t reserved;
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
            "[sbb-wrapper] srnot_ConnectionStateNotification connection=%u state=%d send_used=%u recv_used=%u opposite_buffer=%u disconnect=%d detailed=%u\n",
            (unsigned int)connection_id,
            (int)connection_state,
            (unsigned int)buffer_utilisation.send_buffer_used,
            (unsigned int)buffer_utilisation.receive_buffer_used,
            (unsigned int)opposite_buffer_size,
            (int)disconnect_reason,
            (unsigned int)detailed_disconnect_reason);
    }
}

void srnot_SrDiagnosticNotification(
    const uint32_t connection_id,
    const sraty_ConnectionDiagnosticData connection_diagnostic_data)
{
    (void)connection_diagnostic_data;
    if (sbb_wrapper_udp_trace_enabled()) {
        printf("[sbb-wrapper] srnot_SrDiagnosticNotification connection=%u\n", (unsigned int)connection_id);
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
