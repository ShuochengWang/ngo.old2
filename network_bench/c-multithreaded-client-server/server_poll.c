/*
 * server.c
 * Version 20161003
 * Written by Harry Wong (RedAndBlueEraser)
 */
#define _GNU_SOURCE
#include <sched.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <sys/types.h>
#include <sys/poll.h>
#include <unistd.h>
#include <poll.h>
#include <errno.h>

#define BACKLOG 100
#define TRUE 1
#define FALSE 0

int main(int argc, char *argv[]) {
    cpu_set_t cpus;
    CPU_ZERO(&cpus);
    CPU_SET(1, &cpus);
    // sched_setaffinity(0, sizeof(cpu_set_t), &cpus);

    int port, socket_fd, new_socket_fd;
    struct sockaddr_in address;
    socklen_t client_address_len;
    struct pollfd fds[200];
    int nfds = 1;
    char buf[1024*1024];
    int end_server = FALSE, compress_array = FALSE;

    /* Get port from command line arguments or stdin. */
    port = argc > 1 ? atoi(argv[1]) : 0;
    if (!port) {
        printf("Enter Port: ");
        scanf("%d", &port);
    }

    /* Initialise IPv4 address. */
    memset(&address, 0, sizeof address);
    address.sin_family = AF_INET;
    address.sin_port = htons(port);
    address.sin_addr.s_addr = INADDR_ANY;

    /* Create TCP socket. */
    if ((socket_fd = socket(AF_INET, SOCK_STREAM | SOCK_NONBLOCK, 0)) == -1) {
        perror("socket");
        exit(1);
    }

    /* Bind address to socket. */
    if (bind(socket_fd, (struct sockaddr *)&address, sizeof address) == -1) {
        perror("bind");
        exit(1);
    }

    /* Listen on socket. */
    if (listen(socket_fd, BACKLOG) == -1) {
        perror("listen");
        exit(1);
    }

    memset(fds, 0 , sizeof(fds));
    fds[0].fd = socket_fd;
    fds[0].events = POLLIN;

    int timeout = (3 * 60 * 1000);
    while (1) {
        int ret = poll(fds, nfds, timeout);

        if (ret < 0) {
            perror("poll failed");
            break;
        } else if (ret == 0) {
            perror("poll time out");
            break;
        }

        int current_size = nfds;
        for (int i = 0; i < current_size; ++i) {
            if (fds[i].revents == 0) continue;
            else if (fds[i].revents != POLLIN) {
                printf("revents error: i=%d, revents=%d\n", i, fds[i].revents);
                end_server = TRUE;
                break;
            }

            if (fds[i].fd == socket_fd) {
                int new_socket;
                do {
                    new_socket = accept(socket_fd, NULL, NULL);
                    if (new_socket < 0) {
                        if (errno != EWOULDBLOCK) {
                            perror("accept failed");
                            end_server = TRUE;
                        }
                    }

                    fds[nfds].fd = new_socket;
                    fds[nfds].events = POLLIN;
                    nfds++;
                } while (new_socket != -1);

            } else {
                int close_conn = 0;
                while (1) {
                    int bytes_read = read(fds[i].fd, buf, sizeof(buf));
                    if (bytes_read < 0) {
                        if (errno != EWOULDBLOCK) {
                            perror("read failed");
                            close_conn = 1;
                        }
                        break;
                    } else if (bytes_read == 0) {
                        close_conn = 1;
                        break;
                    }

                    int bytes_write = write(fds[i].fd, buf, bytes_read);
                    if (bytes_write < 0) {
                        perror("write error");
                        close_conn = 1;
                        break;
                    } else if (bytes_write != bytes_read) {
                        printf("bytes_write != bytes_read, %d, %d\n", bytes_write, bytes_read);
                        close_conn = 1;
                        break;
                    }
                }

                if (close_conn) {
                    close(fds[i].fd);
                    fds[i].fd = -1;
                    compress_array = TRUE;
                }
            }
        }

        if (compress_array) {
          compress_array = FALSE;
          for (int i = 0; i < nfds; i++) {
            if (fds[i].fd == -1) {
              for(int j = i; j < nfds; j++) fds[j].fd = fds[j+1].fd;
              i--;
              nfds--;
            }
          }
        }

        if (end_server) break;
    }

    for (int i = 0; i < nfds; i++)
    {
        if (fds[i].fd >= 0) close(fds[i].fd);
    }

    return 0;
}
