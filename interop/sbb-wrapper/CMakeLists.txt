cmake_minimum_required(VERSION 3.16)

project(sbb_wrapper_skeleton C)

set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

set(SBB_ROOT "" CACHE PATH "Path to an external SBB RaSTA stack checkout")

if(SBB_ROOT)
    message(STATUS "SBB_ROOT=${SBB_ROOT}")
else()
    message(WARNING "SBB_ROOT is not set. Building skeleton stubs only; no SBB libraries are linked.")
endif()

add_library(sbb_wrapper_common
    src/ping_pong_payload.c
    src/sbb_adapter.c
    src/udp_transport.c
)

target_include_directories(sbb_wrapper_common
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

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
