cmake_minimum_required(VERSION 3.16)

project(sbb_wrapper_skeleton C)

set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

set(SBB_ROOT "" CACHE PATH "Path to an external SBB RaSTA stack checkout")
option(SBB_WRAPPER_ALLOW_STUBS "Build without the external SBB stack (unit tests only)" OFF)

if(SBB_ROOT AND EXISTS "${SBB_ROOT}/source/modules/rasta_redundancy/CMakeLists.txt")
    message(STATUS "SBB_ROOT=${SBB_ROOT}")
    set(SBB_WRAPPER_HAS_SBB_REDL TRUE)
elseif(SBB_WRAPPER_ALLOW_STUBS)
    message(WARNING "Building explicitly requested skeleton stubs only; no SBB libraries are linked.")
    set(SBB_WRAPPER_HAS_SBB_REDL FALSE)
else()
    message(FATAL_ERROR
        "A real SBB wrapper build requires -DSBB_ROOT=<SBB checkout>. "
        "The checkout must contain source/modules/rasta_redundancy/CMakeLists.txt. "
        "Use -DSBB_WRAPPER_ALLOW_STUBS=ON only for stub-only unit tests.")
endif()

if(SBB_WRAPPER_HAS_SBB_REDL)
    set(_sbb_wrapper_state_machine "${SBB_ROOT}/source/modules/rasta_safety_retransmission/src/srstm_sr_state_machine.c")
    file(READ "${_sbb_wrapper_state_machine}" _sbb_wrapper_state_machine_source)
    string(FIND "${_sbb_wrapper_state_machine_source}" "sbb_wrapper_diag_note_timeout_branch" _sbb_wrapper_timeout_hook_index)
    if(_sbb_wrapper_timeout_hook_index EQUAL -1)
        message(FATAL_ERROR
            "SBB timeout branch instrumentation is missing. Apply interop/sbb-wrapper/sbb-timeout-instrumentation.patch to SBB_ROOT before configuring.")
    endif()
    set(_sbb_wrapper_previous_build_testing ${BUILD_TESTING})
    set(BUILD_TESTING OFF CACHE BOOL "Disable external SBB tests while embedding SBB libraries" FORCE)
    add_subdirectory("${SBB_ROOT}/source/modules/rasta_common" "${CMAKE_BINARY_DIR}/sbb-rasta-common")
    add_subdirectory("${SBB_ROOT}/source/modules/rasta_redundancy" "${CMAKE_BINARY_DIR}/sbb-rasta-redundancy")
    add_subdirectory("${SBB_ROOT}/source/modules/rasta_safety_retransmission" "${CMAKE_BINARY_DIR}/sbb-rasta-safety-retransmission")
    target_include_directories(rasta_safety_retransmission PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src)
    set(BUILD_TESTING ${_sbb_wrapper_previous_build_testing} CACHE BOOL "Restore wrapper testing setting" FORCE)
endif()

add_library(sbb_wrapper_common OBJECT
    src/ping_pong_payload.c
    src/ping_pong_responder.c
    src/rtt_benchmark.c
    src/sbb_adapter.c
    src/sbb_diagnostics.c
    src/sbb_endpoint.c
    src/sbb_redundancy_notifications.c
    src/sbb_safety_notifications.c
    src/sbb_system_adapter.c
    src/udp_transport.c
    src/wrapper_settings.c
)

target_include_directories(sbb_wrapper_common
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

if(SBB_WRAPPER_HAS_SBB_REDL)
    target_compile_definitions(sbb_wrapper_common PUBLIC SBB_WRAPPER_HAS_SBB_REDL=1)
    target_include_directories(sbb_wrapper_common
        PUBLIC
            ${SBB_ROOT}/source/modules/rasta_common/inc
            ${SBB_ROOT}/source/modules/rasta_redundancy/inc
            ${SBB_ROOT}/source/modules/rasta_safety_retransmission/inc
    )
    target_link_libraries(sbb_wrapper_common PUBLIC rasta_safety_retransmission rasta_redundancy rasta_common)
endif()

if(SBB_WRAPPER_HAS_SBB_REDL)
    add_executable(sbb-rasta-wrapper
        src/main.c
    )

    target_link_libraries(sbb-rasta-wrapper PRIVATE sbb_wrapper_common)
endif()

enable_testing()

add_executable(ping_pong_payload_test
    tests/ping_pong_payload_test.c
)

target_link_libraries(ping_pong_payload_test PRIVATE sbb_wrapper_common)

add_test(NAME ping_pong_payload_test COMMAND ping_pong_payload_test)

add_executable(ping_pong_responder_test
    tests/ping_pong_responder_test.c
)

target_link_libraries(ping_pong_responder_test PRIVATE sbb_wrapper_common)

add_test(NAME ping_pong_responder_test COMMAND ping_pong_responder_test)

add_executable(udp_transport_test
    tests/udp_transport_test.c
)

target_link_libraries(udp_transport_test PRIVATE sbb_wrapper_common)

add_test(NAME udp_transport_test COMMAND udp_transport_test)

add_executable(sbb_adapter_bridge_test
    tests/sbb_adapter_bridge_test.c
)

target_link_libraries(sbb_adapter_bridge_test PRIVATE sbb_wrapper_common)

add_test(NAME sbb_adapter_bridge_test COMMAND sbb_adapter_bridge_test)

add_executable(sbb_safretl_smoke_test
    tests/sbb_safretl_smoke_test.c
)

target_link_libraries(sbb_safretl_smoke_test PRIVATE sbb_wrapper_common)

add_test(NAME sbb_safretl_active_startup_test COMMAND sbb_safretl_smoke_test active)
add_test(NAME sbb_safretl_passive_startup_test COMMAND sbb_safretl_smoke_test passive)

add_executable(sbb_transport_notification_test
    tests/sbb_transport_notification_test.c
)

target_link_libraries(sbb_transport_notification_test PRIVATE sbb_wrapper_common)

add_test(NAME sbb_transport_notification_test COMMAND sbb_transport_notification_test)

add_executable(trace_disabled_output_test
    tests/trace_disabled_output_test.c
)

target_link_libraries(trace_disabled_output_test PRIVATE sbb_wrapper_common)

add_test(NAME trace_disabled_output_test COMMAND trace_disabled_output_test)

add_executable(buffered_diagnostics_test
    tests/buffered_diagnostics_test.c
)

target_link_libraries(buffered_diagnostics_test PRIVATE sbb_wrapper_common)

add_test(NAME buffered_diagnostics_test COMMAND buffered_diagnostics_test)

add_executable(wrapper_settings_test
    tests/wrapper_settings_test.c
)

target_link_libraries(wrapper_settings_test PRIVATE sbb_wrapper_common)

add_test(NAME wrapper_settings_test COMMAND wrapper_settings_test)

add_executable(rtt_benchmark_test
    tests/rtt_benchmark_test.c
)

target_link_libraries(rtt_benchmark_test PRIVATE sbb_wrapper_common)

add_test(NAME rtt_benchmark_test COMMAND rtt_benchmark_test)
