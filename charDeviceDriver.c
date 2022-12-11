#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <asm/uaccess.h>
#include <charDeviceDriver.h>

// In case this affects tests
MODULE_LICENSE("GPL");

// Write a device driver for a character device which implements a simple way of message passing.
// The kernel maintains a list of messages.
// To limit memory usage, we impose a limit of 4KB = 4*1024 bytes for each message
// and also impose a limit of the total number of messages stored in the kernel, which is 1000.

// Your device driver should perform the following operations:
// - When the module is loaded, the device is created. An empty list of messages is created as well.
// - Removing the module deallocates all messages, removes the list of messages and removes the device.
// - Reading from the device returns one message, and removes this message from the kernel list.
//   If the list of messages is empty, the reader returns -EAGAIN.
// - Writing to the device stores the message in kernel space and adds it to the list if the message
//   is below the maximum size, and the limit of the number of all messages stored in the kernel
//   wouldn't be surpassed with this message. If the message is too big, -EINVAL is returned,
//   and if the limit of the number of all messages was surpassed, -EBUSY is returned.
// - The kernel module which implements this driver must be called charDeviceDriver.ko.

// You need to ensure that your code deals with multiple attempts at reading and writing at the same time.
// Moreover, your code should handle several read and write attempts concurrently.
// Your critical sections should be as short as possible.
// The reader should obtain the messages in a FIFO (first in first out) manner.

#define MAX_STRING_LENGTH 4096
#define MAX_QUEUE_SIZE 1000

DEFINE_MUTEX(mutex);

typedef struct Queue
{
    char strings[MAX_QUEUE_SIZE][MAX_STRING_LENGTH];
    int sizes[MAX_QUEUE_SIZE];
    int front;
    int end;
    int size;
} Queue;

// Create a queue.
Queue *create_queue(void)
{
    Queue *q = kmalloc(sizeof(Queue), GFP_KERNEL);
    if (q == NULL)
    {
        printk(KERN_ALERT "Error: could not allocate memory for queue\n");
        return NULL;
    }
    q->front = 0;
    q->end = MAX_QUEUE_SIZE - 1;
    q->size = 0;
    return q;
}

// Add a string to the queue.
int enqueue(Queue *queue, char *string, int length)
{
    mutex_lock(&mutex);

    if (queue->size == MAX_QUEUE_SIZE)
    {
        // printk(KERN_INFO "[Queue] Queue is full\n");
        mutex_unlock(&mutex);
        return -1;
    }

    queue->end = (queue->end + 1) % MAX_QUEUE_SIZE;
    memcpy(queue->strings[queue->end], string, length);
    queue->sizes[queue->end] = length;
    queue->size++;

    mutex_unlock(&mutex);

    return 0;
}

// Removes a string from the queue.
int dequeue(Queue *queue, char *string)
{
    int length;

    mutex_lock(&mutex);

    if (queue->size == 0)
    {
        // printk(KERN_INFO "[Queue] Queue is empty\n");
        mutex_unlock(&mutex);
        return -1;
    }

    length = queue->sizes[queue->front];
    memcpy(string, queue->strings[queue->front], length);
    queue->front = (queue->front + 1) % MAX_QUEUE_SIZE;
    queue->size--;

    mutex_unlock(&mutex);

    return length;
}

Queue *queue = NULL;

// This function is called whenever a process tries to do an ioctl on our device file.
// We get two extra parameters (additional to the inode and file structures, which all device functions get):
// the number of the ioctl called and the parameter given to the ioctl function.
// If the ioctl is write or read/write (meaning output is returned to the calling process),
// the ioctl call returns the output of this function.
static long device_ioctl(
    struct file *file,
    unsigned int ioctl_num,
    unsigned long ioctl_param)
{
    printk(KERN_INFO "Sorry, this operation isn't supported\n");
    return -EINVAL;
}

// This function is called when the module is loaded.
int init_module(void)
{
    // When the module is loaded, the device is created. An empty list of messages is created as well.
    queue = create_queue();

    Major = register_chrdev(0, DEVICE_NAME, &fops);

    if (Major < 0)
    {
        printk(KERN_ALERT "Registering char device failed with %d\n", Major);
        return Major;
    }

    // Required for tests
    printk(KERN_INFO "I was assigned major number %d. To talk to\n", Major);
    printk(KERN_INFO "the driver, create a dev file with\n");
    printk(KERN_INFO "'mknod /dev/%s c %d 0'.\n", DEVICE_NAME, Major);
    printk(KERN_INFO "Try various minor numbers. Try to cat and echo to\n");
    printk(KERN_INFO "the device file.\n");
    printk(KERN_INFO "Remove the device file and module when done.\n");

    return SUCCESS;
}

// This function is called when the module is unloaded.
void cleanup_module(void)
{
    // printk(KERN_INFO "Cleaning up module\n");

    // Removing the module deallocates all messages, removes the list of messages and removes the device.
    kfree(queue);
    queue = NULL;

    // Unregister the device
    unregister_chrdev(Major, DEVICE_NAME);
}

// Called when a process tries to open the device file, like `cat /dev/chardev`.
static int device_open(struct inode *inode, struct file *file)
{
    // printk(KERN_INFO "Device opened\n");

    try_module_get(THIS_MODULE);

    return SUCCESS;
}

// Called when a process closes the device file.
static int device_release(struct inode *inode, struct file *file)
{
    // printk(KERN_INFO "Device closed\n");

    module_put(THIS_MODULE);

    return 0;
}

// Called when a process, which already opened the dev file, attempts to read from it.
static ssize_t device_read(
    struct file *filp, // see include/linux/fs.h
    char *buffer,      // buffer to fill with data
    size_t length,     // length of the buffer
    loff_t *offset)
{
    char *item;
    int item_length;
    // printk(KERN_INFO "Device read\n");

    // Reading from the device returns one message, and removes this message from the kernel list.
    // If the list of messages is empty, the reader returns -EAGAIN.

    item = kmalloc(sizeof(char) * MAX_STRING_LENGTH, GFP_KERNEL);
    item_length = dequeue(queue, item);
    if (item_length < 0)
    {
        printk(KERN_INFO "Queue is empty\n");
        return -EAGAIN;
    }
    // printk(KERN_INFO "About to `copy_to_user`\n");
    if (item_length < length)
        length = item_length;
    if (copy_to_user(buffer, item, length))
    {
        printk(KERN_INFO "Failed to `copy_to_user`\n");
        kfree(item);
        return -EFAULT;
    }

    // printk(KERN_INFO "Read from queue\n");
    kfree(item);

    return length;
}

// Called when a process writes to dev file, e.g. `echo "Hello, World!" > /dev/chardev`.
static ssize_t device_write(struct file *filp, const char *buffer, size_t length, loff_t *off)
{
    char *msg;
    int result;
    // printk(KERN_INFO "Device write\n");

    // Writing to the device stores the message in kernel space and adds it to the list
    // if the message is below the maximum size, and the limit of the number of all messages stored in the kernel
    // wouldn't be surpassed with this message. If the message is too big, -EINVAL is returned,
    // and if the limit of the number of all messages was surpassed, -EBUSY is returned.

    if (length > MAX_STRING_LENGTH)
    {
        printk(KERN_INFO "Message too long\n");
        return -EINVAL;
    }

    // Store the message in kernel space and add it to the list
    msg = kmalloc(sizeof(char) * length, GFP_KERNEL);
    if (copy_from_user(msg, buffer, length))
    {
        printk(KERN_INFO "Failed to copy from user\n");
        kfree(msg);
        return -EFAULT;
    }
    result = enqueue(queue, msg, length);
    kfree(msg);

    if (result != 0)
    {
        printk(KERN_INFO "Queue too long\n");
        return -EBUSY;
    }

    // printk(KERN_INFO "Item added to the queue\n");

    return length;
}
