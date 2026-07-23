#ifndef SBB_WRAPPER_SBB_DIAGNOSTICS_H
#define SBB_WRAPPER_SBB_DIAGNOSTICS_H

#include "sbb_adapter.h"

#include <stdint.h>

void sbb_wrapper_diag_set_context(const char *role, uint32_t connection_id, uint32_t sender_id, uint32_t receiver_id);
void sbb_wrapper_diag_set_phase(const char *phase);
void sbb_wrapper_diag_set_debug_no_abort(int enabled);
int sbb_wrapper_diag_debug_no_abort(void);
void sbb_wrapper_diag_record_fatal(radef_RaStaReturnCode reason);
int sbb_wrapper_diag_has_fatal(void);
radef_RaStaReturnCode sbb_wrapper_diag_fatal_reason(void);
void sbb_wrapper_diag_observe_connection_state(int state);
void sbb_wrapper_diag_observe_connection_snapshot(int state, uint16_t send_used, uint16_t recv_used, uint16_t opposite_buffer);
void sbb_wrapper_diag_observe_connection_notification(
    int state,
    uint16_t send_used,
    uint16_t recv_used,
    uint16_t opposite_buffer,
    int disconnect_reason,
    uint16_t detailed_disconnect_reason);
void sbb_wrapper_diag_observe_check_timings_result(radef_RaStaReturnCode result);
void sbb_wrapper_diag_observe_read_data_result(radef_RaStaReturnCode result);
void sbb_wrapper_diag_observe_send_data_result(radef_RaStaReturnCode result);
void sbb_wrapper_diag_observe_protocol_counters(
    uint32_t ec_safety,
    uint32_t ec_address,
    uint32_t ec_type,
    uint32_t ec_sn,
    uint32_t ec_csn);
void sbb_wrapper_diag_observe_successful_ping(uint32_t counter);
void sbb_wrapper_diag_observe_sr_type(uint16_t sr_type);
int sbb_wrapper_diag_has_reached_up(void);
int sbb_wrapper_diag_closed_after_up(void);
uint32_t sbb_wrapper_diag_heartbeat_count(void);
void sbb_wrapper_diag_mark_application_complete(void);
int sbb_wrapper_diag_application_complete(void);
const char *sbb_wrapper_diag_role(void);
uint32_t sbb_wrapper_diag_connection_id(void);
uint32_t sbb_wrapper_diag_sender_id(void);
uint32_t sbb_wrapper_diag_receiver_id(void);
const char *sbb_wrapper_diag_phase(void);

const char *sbb_wrapper_rasta_return_code_name(radef_RaStaReturnCode code);
const char *sbb_wrapper_connection_state_name(int state);
const char *sbb_wrapper_disconnect_reason_name(int reason);
void sbb_wrapper_diag_print_final(
    uint32_t requested_rounds,
    uint32_t received_pings,
    uint32_t sent_pongs,
    uint32_t malformed_or_mismatched,
    int success);

#endif
