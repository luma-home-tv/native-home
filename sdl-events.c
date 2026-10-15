/* Narrow SDL ABI adapter. Built against each target's actual headers: webOS adds fields
 * to keyboard/mouse events, so Rust must not reinterpret the desktop SDL_Event layout. */
#include <SDL.h>
#include <stdint.h>
#include <string.h>
#include <stddef.h>
#include <stdio.h>
typedef struct {uint32_t kind, key, repeat, source; int32_t scan, x, y, wheel; char text[256]; int32_t edit_start, edit_length;} HomeEvent;
_Static_assert(sizeof(HomeEvent)==296,"HomeEvent ABI");
#ifdef NATIVE_HOME_WEBOS
_Static_assert(offsetof(SDL_TextInputEvent,text)==16,"webOS text source field");
#else
_Static_assert(offsetof(SDL_TextInputEvent,text)==12,"desktop text layout");
#endif
static int copy_text(HomeEvent *out,const char *text,size_t capacity) {
    size_t n=0;while(n<capacity && text[n])++n;
    if(n==capacity || n>=sizeof(out->text))return 0;
    memcpy(out->text,text,n);out->text[n]=0;return 1;
}
static int decode_event(const SDL_Event *event,HomeEvent *out) {
    const SDL_Event e=*event;
    memset(out,0,sizeof(*out));
    if(e.type==SDL_TEXTINPUT) {
        out->kind=10;
#ifdef NATIVE_HOME_WEBOS
        out->source=e.text.inputSource;
#endif
        return copy_text(out,e.text.text,sizeof(e.text.text));
    }
    if(e.type==SDL_TEXTEDITING) {
        out->kind=11;out->edit_start=e.edit.start;out->edit_length=e.edit.length;
#ifdef NATIVE_HOME_WEBOS
        out->source=e.edit.inputSource;
#endif
        return copy_text(out,e.edit.text,sizeof(e.edit.text));
    }
#if !defined(NATIVE_HOME_WEBOS) && SDL_VERSION_ATLEAST(2,0,22)
    if(e.type==SDL_TEXTEDITING_EXT) {
        out->kind=11;out->edit_start=e.editExt.start;out->edit_length=e.editExt.length;
        int ok=e.editExt.text && copy_text(out,e.editExt.text,sizeof(out->text));
        SDL_free(e.editExt.text);return ok;
    }
#endif
        if(e.type==SDL_QUIT) {out->kind=1;return 1;}
        if(e.type==SDL_WINDOWEVENT) {
            if(e.window.event==SDL_WINDOWEVENT_CLOSE) {out->kind=1;return 1;}
            if(e.window.event==SDL_WINDOWEVENT_FOCUS_LOST) {out->kind=7;return 1;}
            if(e.window.event==SDL_WINDOWEVENT_HIDDEN||e.window.event==SDL_WINDOWEVENT_MINIMIZED) {out->kind=8;return 1;}
            if(e.window.event==SDL_WINDOWEVENT_FOCUS_GAINED||e.window.event==SDL_WINDOWEVENT_SHOWN||e.window.event==SDL_WINDOWEVENT_RESTORED) {out->kind=9;return 1;}
        }
        if(e.type==SDL_KEYDOWN||e.type==SDL_KEYUP) {
            out->kind=e.type==SDL_KEYDOWN?2:3;out->repeat=e.key.repeat;out->scan=e.key.keysym.scancode;
#ifdef NATIVE_HOME_WEBOS
            out->source=e.key.inputSource;
#endif
            switch(e.key.keysym.sym) {
                case SDLK_UP:out->key=1;break;case SDLK_DOWN:out->key=2;break;
                case SDLK_LEFT:out->key=3;break;case SDLK_RIGHT:out->key=4;break;
                case SDLK_RETURN:case SDLK_KP_ENTER:out->key=5;break;
                case SDLK_ESCAPE:out->key=6;break;
                case SDLK_BACKSPACE:out->key=7;break;
                case SDLK_DELETE:out->key=8;break;
                default:break;
            }
#ifdef NATIVE_HOME_WEBOS
            /* Observed on this webOS24 TV: remote OK 77, Back 482. Do not consume Home,
             * power, volume, settings or channel keys, nor guess unobserved LG scancodes. */
            if(out->scan==77)out->key=5;
            if(out->scan==482)out->key=6;
#endif
            if(out->key)return 1;
        }
        if(e.type==SDL_MOUSEMOTION) {out->kind=4;out->x=e.motion.x;out->y=e.motion.y;return 1;}
        if(e.type==SDL_MOUSEBUTTONUP&&e.button.button==SDL_BUTTON_LEFT) {out->kind=5;out->x=e.button.x;out->y=e.button.y;return 1;}
        if(e.type==SDL_MOUSEWHEEL) {out->kind=6;out->wheel=e.wheel.y*(e.wheel.direction==SDL_MOUSEWHEEL_FLIPPED?-1:1);return 1;}
    return 0;
}
int native_home_poll(HomeEvent *out) {
    SDL_Event event;
    while(SDL_PollEvent(&event)) {if(decode_event(&event,out))return 1;}
    return 0;
}

typedef struct {uint32_t support,active,panel_flag,focused,major,minor,patch,reserved;} HomeKeyboard;
_Static_assert(sizeof(HomeKeyboard)==32,"HomeKeyboard ABI");
void native_home_keyboard_status(void *window,HomeKeyboard *out) {
    SDL_version version;SDL_GetVersion(&version);memset(out,0,sizeof(*out));
    out->support=SDL_HasScreenKeyboardSupport()!=SDL_FALSE;
    out->active=SDL_IsTextInputActive()!=SDL_FALSE;
    out->panel_flag=window && SDL_IsScreenKeyboardShown((SDL_Window*)window)!=SDL_FALSE;
    out->focused=window && (SDL_GetWindowFlags((SDL_Window*)window)&SDL_WINDOW_INPUT_FOCUS)!=0;
    out->major=version.major;out->minor=version.minor;out->patch=version.patch;
}
int native_home_text_start(void *window) {
    if(!window || !(SDL_GetWindowFlags((SDL_Window*)window)&SDL_WINDOW_INPUT_FOCUS))return 0;
#ifdef NATIVE_HOME_WEBOS
    if(!SDL_HasScreenKeyboardSupport())return 0;
#endif
    SDL_StartTextInput();return 1;
}
void native_home_text_flush(void) {SDL_FlushEvents(SDL_TEXTEDITING,SDL_TEXTINPUT);}
void native_home_text_stop(void) {SDL_StopTextInput();SDL_FlushEvents(SDL_TEXTEDITING,SDL_TEXTINPUT);}
/* Both platforms test the actual typed decoder. Never push synthetic host text through sdl2-compat. */
int native_home_test_text_decode(int composition,const char *text,size_t length,HomeEvent *out) {
    SDL_Event e;memset(&e,0,sizeof(e));e.type=composition?SDL_TEXTEDITING:SDL_TEXTINPUT;
    if(length>sizeof(e.text.text))return 0;
    if(composition){memcpy(e.edit.text,text,length);e.edit.start=1;e.edit.length=2;}
    else {memcpy(e.text.text,text,length);}
    return decode_event(&e,out);
}
int native_home_test_text_push(const char *text,size_t length) {
#ifdef NATIVE_HOME_WEBOS
    SDL_Event e;memset(&e,0,sizeof(e));e.type=SDL_TEXTINPUT;
    if(length>=sizeof(e.text.text))return 0;
    memcpy(e.text.text,text,length);return SDL_PushEvent(&e)==1;
#else
    (void)text;(void)length;return 0;
#endif
}

static void text_trace_output(void *userdata,int category,SDL_LogPriority priority,const char *message) {
    (void)userdata;fprintf(stderr,"SDL %d/%d: %s\n",category,(int)priority,message?message:"");fflush(stderr);
}
void native_home_text_trace(void) {SDL_LogSetOutputFunction(text_trace_output,NULL);SDL_LogSetPriority(SDL_LOG_CATEGORY_INPUT,SDL_LOG_PRIORITY_DEBUG);}
