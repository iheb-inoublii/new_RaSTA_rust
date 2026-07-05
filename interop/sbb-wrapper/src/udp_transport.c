#include "udp_transport.h"

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#define SBB_WRAPPER_UDP_CHANNEL_COUNT 2u

typedef struct SbbWrapperUdpRuntimeChannel {
    int fd;
    uint16_t local_port;
    uint16_t remote_port;
    struct sockaddr_in remote_addr;
} SbbWrapperUdpRuntimeChannel;

static SbbWrapperUdpRuntimeChannel g_channels[SBB_WRAPPER_UDP_CHANNEL_COUNT] = {
    {-1, 0u, 0u, {0}},
    {-1, 0u, 0u, {0}},
};
static int g_initialized = 0;
static int g_trace = 0;

void sbb_wrapper_udp_print_config(const SbbWrapperUdpConfig *config)
{
    if (config == 0) {
        puts("[sbb-wrapper] udp config: <null>");
        return;
    }

    printf("[sbb-wrapper] remote_ip=%s\n", config->remote_ip);
    printf("[sbb-wrapper] udp trace=%s\n", config->trace ? "true" : "false");
    printf(
        "[sbb-wrapper] channel0 local=%u remote=%u\n",
        config->channel0.local_port,
        config->channel0.remote_port);
    printf(
        "[sbb-wrapper] channel1 local=%u remote=%u\n",
        config->channel1.local_port,
        config->channel1.remote_port);
}

static SbbWrapperUdpChannel get_config_channel(const SbbWrapperUdpConfig *config, uint32_t channel_id)
{
    if (channel_id == 0u) {
        return config->channel0;
    }
    return config->channel1;
}

static void close_runtime_channel(SbbWrapperUdpRuntimeChannel *channel)
{
    if (channel->fd >= 0) {
        if (g_trace) {
            printf("[sbb-wrapper] udp close: fd=%d local=%u\n", channel->fd, channel->local_port);
        }
        close(channel->fd);
    }
    channel->fd = -1;
    channel->local_port = 0u;
    channel->remote_port = 0u;
    memset(&channel->remote_addr, 0, sizeof(channel->remote_addr));
}

static int set_nonblocking(int fd)
{
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags < 0) {
        return -1;
    }
    return fcntl(fd, F_SETFL, flags | O_NONBLOCK);
}

static int open_runtime_channel(
    const char *remote_ip,
    uint32_t channel_id,
    const SbbWrapperUdpChannel *config,
    SbbWrapperUdpRuntimeChannel *runtime)
{
    struct sockaddr_in local_addr;
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    int reuse = 1;

    if (fd < 0) {
        printf("[sbb-wrapper] udp channel%u socket failed: %s\n", channel_id, strerror(errno));
        return -1;
    }

    if (setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse)) != 0) {
        printf("[sbb-wrapper] udp channel%u setsockopt failed: %s\n", channel_id, strerror(errno));
        close(fd);
        return -1;
    }

    memset(&local_addr, 0, sizeof(local_addr));
    local_addr.sin_family = AF_INET;
    local_addr.sin_addr.s_addr = htonl(INADDR_ANY);
    local_addr.sin_port = htons(config->local_port);

    if (bind(fd, (const struct sockaddr *)&local_addr, sizeof(local_addr)) != 0) {
        printf(
            "[sbb-wrapper] udp channel%u bind local=%u failed: %s\n",
            channel_id,
            config->local_port,
            strerror(errno));
        close(fd);
        return -1;
    }

    if (set_nonblocking(fd) != 0) {
        printf("[sbb-wrapper] udp channel%u nonblocking failed: %s\n", channel_id, strerror(errno));
        close(fd);
        return -1;
    }

    memset(&runtime->remote_addr, 0, sizeof(runtime->remote_addr));
    runtime->remote_addr.sin_family = AF_INET;
    runtime->remote_addr.sin_port = htons(config->remote_port);
    if (inet_pton(AF_INET, remote_ip, &runtime->remote_addr.sin_addr) != 1) {
        printf("[sbb-wrapper] udp channel%u invalid remote ip=%s\n", channel_id, remote_ip);
        close(fd);
        return -1;
    }

    runtime->fd = fd;
    runtime->local_port = config->local_port;
    runtime->remote_port = config->remote_port;

    printf(
        "[sbb-wrapper] udp channel%u socket opened fd=%d local=%u remote=%s:%u\n",
        channel_id,
        fd,
        config->local_port,
        remote_ip,
        config->remote_port);

    return 0;
}

int sbb_wrapper_udp_init(const SbbWrapperUdpConfig *config)
{
    uint32_t i;

    if (config == 0 || config->remote_ip == 0) {
        puts("[sbb-wrapper] udp init: invalid configuration");
        return -1;
    }

    sbb_wrapper_udp_close();
    g_trace = config->trace;

    for (i = 0u; i < SBB_WRAPPER_UDP_CHANNEL_COUNT; i += 1u) {
        SbbWrapperUdpChannel channel = get_config_channel(config, i);
        if (open_runtime_channel(config->remote_ip, i, &channel, &g_channels[i]) != 0) {
            sbb_wrapper_udp_close();
            return -1;
        }
    }

    g_initialized = 1;
    puts("[sbb-wrapper] udp init: real POSIX UDP sockets ready");
    return 0;
}

void sbb_wrapper_udp_close(void)
{
    uint32_t i;
    for (i = 0u; i < SBB_WRAPPER_UDP_CHANNEL_COUNT; i += 1u) {
        close_runtime_channel(&g_channels[i]);
    }
    g_initialized = 0;
}

int sbb_wrapper_udp_is_initialized(void)
{
    return g_initialized;
}

int sbb_wrapper_udp_trace_enabled(void)
{
    return g_trace;
}

static SbbWrapperUdpRuntimeChannel *runtime_channel(uint32_t transport_channel_id)
{
    if (transport_channel_id >= SBB_WRAPPER_UDP_CHANNEL_COUNT) {
        return 0;
    }
    return &g_channels[transport_channel_id];
}

SbbWrapperUdpResult sbb_wrapper_udp_send(uint32_t transport_channel_id, const uint8_t *message, size_t length)
{
    SbbWrapperUdpRuntimeChannel *channel = runtime_channel(transport_channel_id);
    ssize_t sent;

    if (!g_initialized) {
        return SBB_WRAPPER_UDP_NOT_INITIALIZED;
    }
    if (channel == 0) {
        return SBB_WRAPPER_UDP_INVALID_CHANNEL;
    }
    if (message == 0 && length != 0u) {
        return SBB_WRAPPER_UDP_INVALID_PARAMETER;
    }

    sent = sendto(
        channel->fd,
        message,
        length,
        0,
        (const struct sockaddr *)&channel->remote_addr,
        sizeof(channel->remote_addr));
    if (sent < 0 || (size_t)sent != length) {
        printf(
            "[sbb-wrapper] udp send channel=%u length=%zu failed: %s\n",
            transport_channel_id,
            length,
            strerror(errno));
        return SBB_WRAPPER_UDP_OS_ERROR;
    }

    if (g_trace) {
        printf("[sbb-wrapper] udp send channel=%u length=%zu\n", transport_channel_id, length);
    }

    return SBB_WRAPPER_UDP_OK;
}

SbbWrapperUdpResult sbb_wrapper_udp_receive(
    uint32_t transport_channel_id,
    uint8_t *buffer,
    size_t capacity,
    size_t *length)
{
    SbbWrapperUdpRuntimeChannel *channel = runtime_channel(transport_channel_id);
    struct sockaddr_in from_addr;
    socklen_t from_len = sizeof(from_addr);
    ssize_t received;

    if (length != 0) {
        *length = 0u;
    }
    if (!g_initialized) {
        return SBB_WRAPPER_UDP_NOT_INITIALIZED;
    }
    if (channel == 0) {
        return SBB_WRAPPER_UDP_INVALID_CHANNEL;
    }
    if (buffer == 0 || capacity == 0u) {
        return SBB_WRAPPER_UDP_INVALID_PARAMETER;
    }

    memset(&from_addr, 0, sizeof(from_addr));
    received = recvfrom(
        channel->fd,
        buffer,
        capacity,
        MSG_TRUNC,
        (struct sockaddr *)&from_addr,
        &from_len);

    if (received < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            if (g_trace) {
                printf("[sbb-wrapper] udp receive channel=%u no-message\n", transport_channel_id);
            }
            return SBB_WRAPPER_UDP_NO_MESSAGE;
        }

        printf(
            "[sbb-wrapper] udp receive channel=%u failed: %s\n",
            transport_channel_id,
            strerror(errno));
        return SBB_WRAPPER_UDP_OS_ERROR;
    }

    if ((size_t)received > capacity) {
        printf(
            "[sbb-wrapper] udp receive channel=%u datagram too large received=%zd capacity=%zu\n",
            transport_channel_id,
            received,
            capacity);
        return SBB_WRAPPER_UDP_MESSAGE_TOO_LARGE;
    }

    if (length != 0) {
        *length = (size_t)received;
    }

    if (g_trace) {
        char from_ip[INET_ADDRSTRLEN] = {0};
        const char *ip = inet_ntop(AF_INET, &from_addr.sin_addr, from_ip, sizeof(from_ip));
        printf(
            "[sbb-wrapper] udp receive channel=%u length=%zd from=%s:%u\n",
            transport_channel_id,
            received,
            ip == 0 ? "<unknown>" : ip,
            ntohs(from_addr.sin_port));
    }

    return SBB_WRAPPER_UDP_OK;
}
