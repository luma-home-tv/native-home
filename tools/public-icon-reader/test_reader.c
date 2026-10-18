/* Host fixtures only. This build has no production CLI or deployable test root. */
#define PIR_TESTING 1
#include "reader.c"
#include <limits.h>

static const char *base;
static int point_to_change;
static const char *change;
static void path(char *dst, const char *suffix) {
    int n = snprintf(dst, PATH_MAX, "%s%s", base, suffix);
    if (n < 0 || n >= PATH_MAX) abort();
}
static void file(const char *name, size_t size, unsigned char first) {
    int fd = open(name, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0644);
    unsigned char data[4096] = {137,80,78,71,13,10,26,10};
    if (fd < 0) abort();
    data[0] = first;
    for (size_t done = 0; done < size;) {
        size_t n = size - done > sizeof(data) ? sizeof(data) : size - done;
        if (write(fd, data, n) != (ssize_t)n) abort();
        done += n;
        memset(data, 0, sizeof(data));
    }
    if (close(fd)) abort();
}
static void modify(int point) {
    char a[PATH_MAX], b[PATH_MAX];
    if (point != point_to_change) return;
    if (!strcmp(change, "parent-symlink")) {
        path(a, "/com.example.app"); path(b, "/old-app");
        if (rename(a,b) || symlink("/private-account-not-public",a)) abort();
    } else if (!strcmp(change, "root-replaced")) {
        int n=snprintf(b,sizeof(b),"%s-old",base);
        if(n<0 || n>=(int)sizeof(b) || rename(base,b) || mkdir(base,0700))abort();
    } else if (!strcmp(change, "file-symlink")) {
        path(a, "/com.example.app/icon.png");
        if (unlink(a) || symlink("/private-account-not-public",a)) abort();
    } else if (!strcmp(change, "named-replaced")) {
        path(a, "/replacement.png");file(a,32,137);
        path(b, "/com.example.app/icon.png");if(rename(a,b))abort();
    } else if (!strcmp(change, "grow")) {
        path(a, "/com.example.app/icon.png");int fd=open(a,O_WRONLY|O_APPEND);
        if(fd<0 || write(fd,"x",1)!=1 || close(fd))abort();
    } else if (!strcmp(change, "rewrite-restored")) {
        path(a, "/com.example.app/icon.png");int fd=open(a,O_WRONLY);
        if(fd<0 || pwrite(fd,"x",1,0)!=1 || pwrite(fd,"\211",1,0)!=1 || close(fd))abort();
        struct timespec stamp[2]={{100,0},{100,0}};
        if(utimensat(AT_FDCWD,a,stamp,0))abort();
    }
    test_hook=NULL;
}
static void pause_hook(int point) {if(point==2)sleep(10);}
int main(int argc,char **argv) {
    struct chain c; char app[PATH_MAX],icon[PATH_MAX],other[PATH_MAX];
    const char *id="com.example.app", *relative="icon.png";
    size_t size=32; int expected=0, anchor_result; uid_t uid=getuid(); char too_long[REL_CAP+2];
    if(argc!=3)return 90;
    base=argv[1];change=argv[2];path(app,"/com.example.app");path(icon,"/com.example.app/icon.png");
    if(chmod(base,0755)||mkdir(app,0755))return 91;
    if(!strcmp(change,"oversize"))size=ICON_CAP+1;
    if(!strcmp(change,"max"))size=ICON_CAP;
    if(!strcmp(change,"empty"))size=0;
    if(!strcmp(change,"fifo")){if(mkfifo(icon,0600))abort();}
    else file(icon,size,!strcmp(change,"wrong-signature")?0:137);
    if(!strcmp(change,"hardlink")){path(other,"/alias");if(link(icon,other))abort();}
    if(!strcmp(change,"private-nested")){path(other,"/com.example.app/private");if(mkdir(other,0700))abort();path(other,"/com.example.app/private/icon.png");file(other,32,137);relative="private/icon.png";}
    if(!strcmp(change,"private-root")){if(chmod(base,0700))abort();}
    if(!strcmp(change,"private-app")){if(chmod(app,0700))abort();}
    if(!strcmp(change,"private-file")){if(chmod(icon,0600))abort();}
    if(!strcmp(change,"special")){if(chmod(icon,04600))abort();}
    if(!strcmp(change,"plain777")){if(chmod(app,0777)||chmod(icon,0777))abort();}
    if(!strcmp(change,"jpeg")){if(unlink(icon))abort();path(icon,"/com.example.app/icon.JPEG");file(icon,32,255);int fd=open(icon,O_WRONLY);unsigned char j[3]={255,216,255};if(fd<0||write(fd,j,3)!=3||close(fd))abort();relative="icon.JPEG";}
    if(!strcmp(change,"relative-link")){path(other,"/com.example.app/sub");if(symlink("/private-account-not-public",other))abort();relative="sub/icon.png";}
    if(!strcmp(change,"id-too-long")){memset(too_long,'a',181);too_long[181]=0;id=too_long;}
    if(!strcmp(change,"relative-too-long")){memset(too_long,'a',REL_CAP+1);too_long[REL_CAP+1]=0;relative=too_long;}
    if(!strcmp(change,"many-components"))relative="a/b/c/d/e/f/g/h/icon.png";
    if(!strcmp(change,"control"))relative="icon\n.png";
    if(!strcmp(change,"invalid-id"))id="..";
    if(!strcmp(change,"hidden-id"))id=".ssh";
    if(!strcmp(change,"invalid-relative"))relative="../icon.png";
    if(!strcmp(change,"absolute-relative"))relative="/root/.ssh/icon.png";
    if(!strcmp(change,"uri"))relative="file:icon.png";
    if(!strcmp(change,"trailing-slash"))relative="icon.png/";
    if(!strcmp(change,"wrong-owner"))uid++;
    if(!strcmp(change,"parent-symlink")||!strcmp(change,"root-replaced")||!strcmp(change,"file-symlink"))point_to_change=1;
    if(!strcmp(change,"named-replaced")||!strcmp(change,"grow")||!strcmp(change,"rewrite-restored"))point_to_change=3;
    if(!strcmp(change,"late-replaced")){change="named-replaced";point_to_change=4;}
    if(!strcmp(change,"before-open-replaced")){change="named-replaced";point_to_change=5;}
    if(!strcmp(change,"grow-before-read")){change="grow";point_to_change=2;}
    if(point_to_change)test_hook=modify;
    if(!strcmp(change,"timeout")||!strcmp(change,"timeout-blocked")){
        if(!strcmp(change,"timeout-blocked")){sigset_t set;if(sigemptyset(&set)||sigaddset(&set,SIGALRM)||sigprocmask(SIG_BLOCK,&set,NULL))abort();}
        if(start_deadline(1))abort();test_hook=pause_hook;
    }
    expected=!strcmp(change,"good")||!strcmp(change,"max")||!strcmp(change,"plain777")||!strcmp(change,"jpeg");
    FILE *output=tmpfile();if(!output)return 92;
    anchor_result=anchor(&c,base,uid,getgid());
    int result=anchor_result? -1:read_emit(&c,"developer_apps",id,relative,fileno(output));
    close_chain(&c);
    if(fflush(output)||fseek(output,0,SEEK_END))abort();long length=ftell(output);
    if((expected && result)||( !expected && (result==0 || length!=0)))return 93;
    if(expected){if(fseek(output,0,SEEK_SET))abort();unsigned char b[4096];size_t n;while((n=fread(b,1,sizeof(b),output)))if(fwrite(b,1,n,stdout)!=n)abort();}
    fclose(output);return 0;
}
