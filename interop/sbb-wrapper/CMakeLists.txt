cmake_minimum_required(VERSION 3.16)

project(sbb_wrapper_skeleton C)

set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

set(SBB_ROOT "" CACHE PATH "Path to an external SBB RaSTA stack checkout")

if(SBB_ROOT AND EXISTS "${SBB_ROOT}/source/modules/rasta_redundancy/CMakeLists.txt")
    message(STATUS "SBB_ROOT=${SBB_ROOT}")
    set(SBB_WRAPPER_HAS_SBB_REDL TRUE)
else()
    message(WARNING "SBB_ROOT is not set. Building skeleton stubs only; no SBB libraries are linked.")
    set(SBB_WRAPPER_HAS_SBB_REDL FALSE)
endif()

if(SBB_WRAPPER_HAS_SBB_REDL)
    set(_sbb_wrapper_previous_build_testing ${BUILD_TESTING})
    set(BUILD_TESTING OFF CACHE BOOL "Disable external SBB tests while embedding SBB libraries" FORCE)
    add_subdirectory("${SBB_ROOT}/source/modules/rasta_common" "${CMAKE_BINARY_DIR}/sbb-rasta-common")
    add_subdirectory("${SBB_ROOT}/source/modules/rasta_redundancy" "${CMAKE_BINARY_DIR}/sbb-rasta-redundancy")
    add_subdirectory("${SBB_ROOT}/source/modules/rasta_safety_retransmission" "${CMAKE_BINARY_DIR}/sbb-rasta-safety-retransmission")
    set(BUILD_TESTING ${_sbb_wrapper_previous_build_testing} CACHE BOOL "Restore wrapper testing setting" FORCE)
endif()

add_library(sbb_wrapper_common OBJECT
    src/ping_pong_payload.c
    src/sbb_adapter.c
    src/sbb_endpoint.c
    src/sbb_redundancy_notifications.c
    src/sbb_safety_notifications.c
    src/sbb_system_adapter.c
    src/udp_transport.c
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

add_executable(sbb-rasta-wrapper
    src/main.c
)

target_link_libraries(sbb-rasta-wrapper PRIVATE sbb_wrapper_common)

enable_testing()

add_executable(ping_pong_payload_test
    tests/ping_pong_payload_test.c
)

target_link_libraries(ping_pong_payload_test PRIVATE sbb_wrapper_common)

add_test(NAME ping_pong_payload_test COMMAND ping_pong_payload_test)

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

add_test(NAME sbb_safretl_smoke_test COMMAND sbb_safretl_smoke_test)
