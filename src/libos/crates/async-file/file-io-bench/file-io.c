/*
 * server.c
 * Version 20161003
 * Written by Harry Wong (RedAndBlueEraser)
 */
#define _GNU_SOURCE
#include <sched.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <assert.h>
#include <sys/time.h>

typedef struct pthread_arg_t {
    void* buf;
    double duration;
    long process_bytes;
} pthread_arg_t;


int thread_num;
int file_num;
long file_block_size;
int file_req_merge_num;
long file_total_size;
int is_read;
int is_seq;
int use_fsync;
int use_direct;
int loops;

long position;
int current_file;


int* fds;
pthread_t* tid;
pthread_arg_t* pthread_args;
pthread_mutex_t lock; 

const double mb_size = 1024 * 1024;
const double kb_size = 1024;

/* Thread routine to serve connection to client. */
void *pthread_routine(void *arg);

void prepare() {
    position = 0;
    current_file = 0;

    fds = (int*)malloc(file_num * sizeof(int));
    void* buf = NULL;
    posix_memalign(&buf, 4096, file_block_size);
    assert(buf !=NULL);
    char file_name[512];
    long file_size = file_total_size / file_num;
    for (int i = 0; i < file_num; ++i) {
        snprintf(file_name, sizeof(file_name), "test_file.%d", i);
        int flags = O_RDWR | O_CREAT | O_TRUNC;
        if (use_direct) flags |= O_DIRECT;
        int fd = open(file_name, flags, S_IRUSR | S_IRUSR);
        if (fd < 0) {
            perror("open");
            return;
        }
        fds[i] = fd;

        for (int j = 0; j < file_size; j += file_block_size) {
            int ret = write(fd, buf, file_block_size);
            if (ret != file_block_size) {
                printf("ret: %d, fd: %d\n", ret, fd);
                perror("write");
            }
            assert(ret == file_block_size);
        }
    }
    free(buf);

    tid = (pthread_t*)malloc(thread_num * sizeof(pthread_t));
    pthread_args = (pthread_arg_t*)malloc(thread_num * sizeof(pthread_arg_t));
    for (int i = 0; i < thread_num; ++i) {
        posix_memalign(&pthread_args[i].buf, 4096, file_block_size);
        assert(pthread_args[i].buf != NULL);
        pthread_args[i].process_bytes = 0;
        pthread_args[i].duration = 0;
    }
}

void main_loop() {
    pthread_attr_t pthread_attr;
    if (pthread_attr_init(&pthread_attr) != 0) {
        perror("pthread_attr_init");
        exit(1);
    }

    cpu_set_t cpus;
    CPU_ZERO(&cpus);
    CPU_SET(1, &cpus);
    // sched_setaffinity(0, sizeof(cpu_set_t), &cpus);
    pthread_attr_setaffinity_np(&pthread_attr, sizeof(cpu_set_t), &cpus);

    if (pthread_mutex_init(&lock, NULL) != 0) { 
        printf("\n mutex init has failed\n"); 
        return; 
    }


    struct timeval tv1;
    gettimeofday(&tv1, NULL);

    for (int i = 0; i < thread_num; ++i) {
        /* Create thread to serve connection to client. */
        if (pthread_create(&tid[i], &pthread_attr, pthread_routine, (void *)&pthread_args[i]) != 0) {
            perror("pthread_create");
            continue;
        }
    }

    double avg_time = 0;
    long total_process_bytes = 0;
    for (int i = 0; i < thread_num; i++) {
       pthread_join(tid[i], NULL);
       avg_time += pthread_args[i].duration;
       total_process_bytes += pthread_args[i].process_bytes;
    }
    avg_time /= thread_num;

    struct timeval tv2;
    gettimeofday(&tv2, NULL);
    double duration = (double) (tv2.tv_usec - tv1.tv_usec) / 1000000 + (double) (tv2.tv_sec - tv1.tv_sec);

    double throughput = total_process_bytes / mb_size / duration;
    double avg_throughput = total_process_bytes / mb_size / avg_time;

    printf("duration: %f s (avg_time: %f s), throughput: %f MB/s (avg_throughput: %f MB/s)\n",
        duration, avg_time, throughput, avg_throughput);
}

void done() {
    for (int i = 0; i < file_num; ++i) {
        close(fds[i]);
    }
    free(fds);

    for (int i = 0; i < thread_num; ++i) {
        free(pthread_args[i].buf);
    }
    free(tid);
    free(pthread_args);
    pthread_mutex_destroy(&lock); 
}

int main(int argc, char *argv[]) {
    thread_num = argc > 1 ? atoi(argv[1]) : 1;
    file_num = argc > 2 ? atoi(argv[2]) : 1;
    file_block_size = argc > 3 ? atoi(argv[3]) : 4; // KB
    file_block_size *= kb_size;
    file_req_merge_num = argc > 4 ? atoi(argv[4]) : 10;
    file_total_size = argc > 5 ? atoi(argv[5]) : 40; // MB
    file_total_size *= mb_size;

    is_read = argc > 6 ? atoi(argv[6]) : 1;
    is_seq = argc > 7 ? atoi(argv[7]) : 1;
    use_fsync = argc > 8 ? atoi(argv[8]) : 1;
    use_direct = argc > 9 ? atoi(argv[9]) : 1;
    loops = argc > 10 ? atoi(argv[10]) : 1;

    printf("thread_num: %d, file_num: %d, file_block_size: %ld, file_req_merge_num: %d, file_total_size: %ld, ",
        thread_num, file_num, file_block_size, file_req_merge_num, file_total_size);
    printf("is_read: %d, is_seq: %d, use_fsync: %d, use_direct: %d, loop: %d. ",
        is_read, is_seq, use_fsync, use_direct, loops);

    prepare();

    main_loop();

    done();
    
    return 0;
}

int seed = 0;
u_int32_t get_random()
{
    u_int32_t hi, lo;
    hi = (seed = seed * 1103515245 + 12345) >> 16;
    lo = (seed = seed * 1103515245 + 12345) >> 16;
    return (hi << 16) + lo;
}

int get_next_request(int* fd, long* offset, int* size) {
    pthread_mutex_lock(&lock);

    long file_size = file_total_size / file_num;

    // if (current_file == file_num) {
    //     current_file = 0;
    //     position = 0;
    // }

    // if (is_seq) {

    // }
    // else {

    // }

    pthread_mutex_unlock(&lock); 
}

void *pthread_routine(void *arg) {
    pthread_arg_t *pthread_arg = (pthread_arg_t *)arg;
    char* buf = (char*)pthread_arg->buf;

    struct timeval tv1;
    gettimeofday(&tv1, NULL);

    // int fd;
    // long offset;
    // int size;
    // while (get_next_request(&fd, &offset, &size)) {
    //     for (int i = 0; i < size; i += file_block_size) {
    //         int cur_size = i + file_block_size <= size ? file_block_size : size - i;
    //         if (is_read) {
    //             int bytes_read = pread(fd, buf, cur_size, offset + i);
    //             if (bytes_read < 0) {
    //                 perror("read return error.");
    //                 break;
    //             }
    //         } else {
    //             char val = buf[0] + 1;
    //             for (int i = 0; i < file_block_size; ++i) buf[i] = val;

    //             int bytes_write = pwrite(fd, buf, cur_size, offset + i);
    //             if (bytes_read < 0) {
    //                 perror("read return error.");
    //                 break;
    //             }
    //         }
    //     }
        
    // }

    long file_size = file_total_size / file_num;
    int fd = fds[0];

    for (int i = 0; i < loops; ++i) {
        if (is_read) {
            if (is_seq) {
                for (long offset = 0; offset < file_size; offset += file_block_size) {
                    int bytes_read = pread(fd, buf, file_block_size, offset);
                    if (bytes_read < 0) {
                        perror("read return error.");
                        break;
                    }
                    pthread_arg->process_bytes += bytes_read;
                }
            } else {
                long cnt = 0;
                int block_num = file_size / file_block_size;
                while (cnt < file_size) {
                    int offset = (get_random() % block_num) * file_block_size;
                    int bytes_read = pread(fd, buf, file_block_size, offset);
                    if (bytes_read < 0) {
                        perror("read return error.");
                        break;
                    }
                    cnt += bytes_read;
                }
                pthread_arg->process_bytes += cnt;
            }
        }
        else {
            if (is_seq) {
                for (long offset = 0; offset < file_size; offset += file_block_size) {
                    char val = buf[0] + 1;
                    for (int i = 0; i < file_block_size; ++i) buf[i] = val;

                    int bytes_write = pwrite(fd, buf, file_block_size, offset);
                    if (bytes_write < 0) {
                        perror("read return error.");
                        break;
                    }
                    pthread_arg->process_bytes += bytes_write;
                }
            }
            else {
                long cnt = 0;
                int block_num = file_size / file_block_size;
                while (cnt < file_size) {
                    char val = buf[0] + 1;
                    for (int i = 0; i < file_block_size; ++i) buf[i] = val;

                    int offset = (get_random() % block_num) * file_block_size;
                    int bytes_write = pwrite(fd, buf, file_block_size, offset);
                    if (bytes_write < 0) {
                        perror("read return error.");
                        break;
                    }
                    cnt += bytes_write;
                }
                pthread_arg->process_bytes += cnt;
            }

            if (use_fsync) fsync(fd);
        }
    }

    struct timeval tv2;
    gettimeofday(&tv2, NULL);
    pthread_arg->duration = (double) (tv2.tv_usec - tv1.tv_usec) / 1000000 + (double) (tv2.tv_sec - tv1.tv_sec);

    return NULL;
}