#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/fs.h>
#include <asm/uaccess.h>
#include <charDeviceDriver.h>

// In case this affects tests
MODULE_LICENSE("GPL");

DEFINE_MUTEX(devLock);

/*
 * This function is called whenever a process tries to do an ioctl on our
 * device file. We get two extra parameters (additional to the inode and file
 * structures, which all device functions get): the number of the ioctl called
 * and the parameter given to the ioctl function.
 *
 * If the ioctl is write or read/write (meaning output is returned to the
 * calling process), the ioctl call returns the output of this function.
 *
 */

static long device_ioctl(
    struct file *file,
    unsigned int ioctl_num,
    unsigned long ioctl_param)
{
    printk(KERN_ALERT "Sorry, this operation isn't supported.\n");
    return -EINVAL;
}

/*
 * This function is called when the module is loaded
 */
int init_module(void)
{
    Major = register_chrdev(0, DEVICE_NAME, &fops);

    if (Major < 0)
    {
        printk(KERN_ALERT "Registering char device failed with %d\n", Major);
        return Major;
    }

    printk(KERN_INFO "I was assigned major number %d. To talk to\n", Major);
    printk(KERN_INFO "the driver, create a dev file with\n");
    printk(KERN_INFO "'mknod /dev/%s c %d 0'.\n", DEVICE_NAME, Major);
    printk(KERN_INFO "Try various minor numbers. Try to cat and echo to\n");
    printk(KERN_INFO "the device file.\n");
    printk(KERN_INFO "Remove the device file and module when done.\n");

    return SUCCESS;
}

/*
 * This function is called when the module is unloaded
 */
void cleanup_module(void)
{
    /* Unregister the device */
    unregister_chrdev(Major, DEVICE_NAME);
}

/*
 * Methods
 */

/* Called when a process tries to open the device file, like `cat /dev/chardev` */
static int device_open(struct inode *inode, struct file *file)
{
    mutex_lock(&devLock);
    if (Device_Open)
    {
        mutex_unlock(&devLock);
        return -EBUSY;
    }
    Device_Open++;
    mutex_unlock(&devLock);
    sprintf(msg, "I already told you %d times Hello world!\n", counter++);
    try_module_get(THIS_MODULE);

    return SUCCESS;
}

/* Called when a process closes the device file. */
static int device_release(struct inode *inode, struct file *file)
{
    mutex_lock(&devLock);
    Device_Open--; /* We're now ready for our next caller */
    mutex_unlock(&devLock);
    /*
     * Decrement the usage count, or else once you opened the file, you'll
     * never get get rid of the module.
     */
    module_put(THIS_MODULE);

    return 0;
}

/* Called when a process, which already opened the dev file, attempts to read from it. */
static ssize_t device_read(
    struct file *filp, /* see include/linux/fs.h */
    char *buffer,      /* buffer to fill with data */
    size_t length,     /* length of the buffer */
    loff_t *offset)
{
    /* Result of function calls */
    int result;

    /*
     * Actually put the data into the buffer
     */
    if (strlen(msg) + 1 < length)
        length = strlen(msg) + 1;
    result = copy_to_user(buffer, msg, length);
    if (result > 0)
        return -EFAULT; /* copy failed */
    /* Most read functions return the number of bytes put into the buffer */
    return length;
}

/* Called when a process writes to dev file, e.g. `echo "Hello, World!" > /dev/chardev` */
static ssize_t
device_write(struct file *filp, const char *buff, size_t len, loff_t *off)
{
    printk(KERN_ALERT "Sorry, this operation isn't supported.\n");
    return -EINVAL;
}
