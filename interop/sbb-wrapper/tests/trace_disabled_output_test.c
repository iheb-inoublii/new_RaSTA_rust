#define _POSIX_C_SOURCE 200809L

#include "sbb_adapter.h"
#include "udp_transport.h"

#include <stdint.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
    SbbWrapperUdpConfig config = {
        .remote_ip = "127.0.0.1",
        .trace = 0,
        .channel0 = {.local_port = 39500u, .remote_port = 39501u},
        .channel1 = {.local_port = 39501u, .remote_port = 39500u},
    };
    uint8_t payload[5] = {0x03u, 1u, 0u, 0u, 0u};
    uint8_t buffer[32] = {0};
    uint16_t length = 0u;
    FILE *capture = tmpfile();
    int saved_stdout;
    long output_size;

    if (capture == 0) {
        return 1;
    }
    saved_stdout = dup(STDOUT_FILENO);
    if (saved_stdout < 0 || dup2(fileno(capture), STDOUT_FILENO) < 0) {
        fclose(capture);
        return 2;
    }

    if (sbb_wrapper_udp_init(&config) != 0) {
        return 3;
    }
    redtri_Init();
    (void)redtri_ReadMessage(0u, (uint16_t)sizeof(buffer), &length, buffer);
    redtri_SendMessage(0u, (uint16_t)sizeof(payload), payload);
    (void)sbb_wrapper_transport_poll_channel(1u);
    (void)redtri_ReadMessage(1u, (uint16_t)sizeof(buffer), &length, buffer);
    sbb_wrapper_udp_close();

    fflush(stdout);
    output_size = ftell(capture);
    (void)dup2(saved_stdout, STDOUT_FILENO);
    close(saved_stdout);
    fclose(capture);

    return output_size == 0L ? 0 : 4;
}
