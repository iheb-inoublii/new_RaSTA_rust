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
const char *sbb_wrapper_diag_role(void);
uint32_t sbb_wrapper_diag_connection_id(void);
uint32_t sbb_wrapper_diag_sender_id(void);
uint32_t sbb_wrapper_diag_receiver_id(void);
const char *sbb_wrapper_diag_phase(void);

const char *sbb_wrapper_rasta_return_code_name(radef_RaStaReturnCode code);
const char *sbb_wrapper_connection_state_name(int state);
const char *sbb_wrapper_disconnect_reason_name(int reason);

#endif
