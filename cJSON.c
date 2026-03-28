/*
  Copyright (c) 2009-2017 Dave Gamble and cJSON contributors
  Minimal subset for parsing JSON. See full cJSON for complete implementation.
*/

#include <string.h>
#include <stdio.h>
#include <math.h>
#include <stdlib.h>
#include <limits.h>
#include <ctype.h>
#include <float.h>

#include "cJSON.h"

/* Internal hooks */
static void *(*global_malloc)(size_t sz) = malloc;
static void (*global_free)(void *ptr) = free;

void cJSON_InitHooks(cJSON_Hooks* hooks) {
    if (!hooks) { global_malloc = malloc; global_free = free; return; }
    global_malloc = hooks->malloc_fn ? hooks->malloc_fn : malloc;
    global_free = hooks->free_fn ? hooks->free_fn : free;
}

static cJSON *cJSON_New_Item(void) {
    cJSON *node = (cJSON*)global_malloc(sizeof(cJSON));
    if (node) memset(node, 0, sizeof(cJSON));
    return node;
}

void cJSON_Delete(cJSON *item) {
    cJSON *next;
    while (item) {
        next = item->next;
        if (!(item->type & cJSON_IsReference) && item->child)
            cJSON_Delete(item->child);
        if (!(item->type & cJSON_IsReference) && item->valuestring)
            global_free(item->valuestring);
        if (!(item->type & cJSON_StringIsConst) && item->string)
            global_free(item->string);
        global_free(item);
        item = next;
    }
}

/* Parser */
typedef struct { const char *content; size_t offset; size_t length; } parse_buffer;

#define can_access(b, idx) ((b) && ((b)->offset + (idx) < (b)->length))
#define buffer_at(b, idx) ((b)->content[(b)->offset + (idx)])

static const char *skip_whitespace(parse_buffer *buf) {
    if (!buf || !buf->content) return NULL;
    while (can_access(buf, 0) && (buffer_at(buf, 0) <= 32))
        buf->offset++;
    if (buf->offset >= buf->length) { buf->offset--; return NULL; }
    return buf->content + buf->offset;
}

static int parse_number(cJSON *item, parse_buffer *buf) {
    double number = 0;
    char *after = NULL;
    if (!can_access(buf, 0)) return 0;
    number = strtod((const char*)buf->content + buf->offset, &after);
    if ((const char*)buf->content + buf->offset == after) return 0;
    item->valuedouble = number;
    item->valueint = (number >= INT_MAX) ? INT_MAX : ((number <= INT_MIN) ? INT_MIN : (int)number);
    item->type = cJSON_Number;
    buf->offset = (size_t)(after - buf->content);
    return 1;
}

static unsigned char parse_hex4(const char *str) {
    unsigned int h = 0;
    for (int i = 0; i < 4; i++) {
        char c = str[i];
        if (c >= '0' && c <= '9') h = (h << 4) | (c - '0');
        else if (c >= 'a' && c <= 'f') h = (h << 4) | (10 + c - 'a');
        else if (c >= 'A' && c <= 'F') h = (h << 4) | (10 + c - 'A');
        else return 0;
    }
    return h;
}

static int parse_string(cJSON *item, parse_buffer *buf) {
    if (!can_access(buf, 0) || buffer_at(buf, 0) != '\"') return 0;
    buf->offset++;
    const char *start = buf->content + buf->offset;
    /* find end */
    size_t len = 0;
    while (can_access(buf, len) && buffer_at(buf, len) != '\"') {
        if (buffer_at(buf, len) == '\\') len++;
        len++;
    }
    /* allocate output (may be shorter due to escapes) */
    char *out = (char*)global_malloc(len + 1);
    if (!out) return 0;
    size_t oi = 0;
    for (size_t i = 0; i < len; i++) {
        if (start[i] == '\\' && i + 1 < len) {
            i++;
            switch (start[i]) {
                case 'b': out[oi++] = '\b'; break;
                case 'f': out[oi++] = '\f'; break;
                case 'n': out[oi++] = '\n'; break;
                case 'r': out[oi++] = '\r'; break;
                case 't': out[oi++] = '\t'; break;
                case '\"': case '\\': case '/': out[oi++] = start[i]; break;
                default: out[oi++] = start[i]; break;
            }
        } else {
            out[oi++] = start[i];
        }
    }
    out[oi] = '\0';
    buf->offset += len + 1; /* skip closing quote */
    item->valuestring = out;
    item->type = cJSON_String;
    return 1;
}

static int parse_value(cJSON *item, parse_buffer *buf);

static int parse_array(cJSON *item, parse_buffer *buf) {
    if (!can_access(buf, 0) || buffer_at(buf, 0) != '[') return 0;
    buf->offset++;
    item->type = cJSON_Array;
    skip_whitespace(buf);
    if (can_access(buf, 0) && buffer_at(buf, 0) == ']') { buf->offset++; return 1; }

    cJSON *child = cJSON_New_Item();
    if (!child) return 0;
    item->child = child;
    skip_whitespace(buf);
    if (!parse_value(child, buf)) return 0;
    skip_whitespace(buf);

    while (can_access(buf, 0) && buffer_at(buf, 0) == ',') {
        buf->offset++;
        skip_whitespace(buf);
        cJSON *new_item = cJSON_New_Item();
        if (!new_item) return 0;
        child->next = new_item;
        new_item->prev = child;
        child = new_item;
        if (!parse_value(child, buf)) return 0;
        skip_whitespace(buf);
    }

    if (!can_access(buf, 0) || buffer_at(buf, 0) != ']') return 0;
    buf->offset++;
    return 1;
}

static int parse_object(cJSON *item, parse_buffer *buf) {
    if (!can_access(buf, 0) || buffer_at(buf, 0) != '{') return 0;
    buf->offset++;
    item->type = cJSON_Object;
    skip_whitespace(buf);
    if (can_access(buf, 0) && buffer_at(buf, 0) == '}') { buf->offset++; return 1; }

    cJSON *child = cJSON_New_Item();
    if (!child) return 0;
    item->child = child;
    skip_whitespace(buf);
    /* parse key */
    cJSON key_item = {0};
    if (!parse_string(&key_item, buf)) { global_free(child); item->child = NULL; return 0; }
    child->string = key_item.valuestring;
    skip_whitespace(buf);
    if (!can_access(buf, 0) || buffer_at(buf, 0) != ':') return 0;
    buf->offset++;
    skip_whitespace(buf);
    if (!parse_value(child, buf)) return 0;
    skip_whitespace(buf);

    while (can_access(buf, 0) && buffer_at(buf, 0) == ',') {
        buf->offset++;
        skip_whitespace(buf);
        cJSON *new_item = cJSON_New_Item();
        if (!new_item) return 0;
        child->next = new_item;
        new_item->prev = child;
        child = new_item;
        cJSON ki2 = {0};
        if (!parse_string(&ki2, buf)) return 0;
        child->string = ki2.valuestring;
        skip_whitespace(buf);
        if (!can_access(buf, 0) || buffer_at(buf, 0) != ':') return 0;
        buf->offset++;
        skip_whitespace(buf);
        if (!parse_value(child, buf)) return 0;
        skip_whitespace(buf);
    }

    skip_whitespace(buf);
    if (!can_access(buf, 0) || buffer_at(buf, 0) != '}') return 0;
    buf->offset++;
    return 1;
}

static int parse_value(cJSON *item, parse_buffer *buf) {
    if (!buf || !buf->content) return 0;
    skip_whitespace(buf);
    if (!can_access(buf, 0)) return 0;

    if (can_access(buf, 3) && strncmp(buf->content + buf->offset, "null", 4) == 0) {
        item->type = cJSON_NULL; buf->offset += 4; return 1;
    }
    if (can_access(buf, 3) && strncmp(buf->content + buf->offset, "true", 4) == 0) {
        item->type = cJSON_True; item->valueint = 1; buf->offset += 4; return 1;
    }
    if (can_access(buf, 4) && strncmp(buf->content + buf->offset, "false", 5) == 0) {
        item->type = cJSON_False; buf->offset += 5; return 1;
    }
    if (buffer_at(buf, 0) == '\"') return parse_string(item, buf);
    if (buffer_at(buf, 0) == '-' || (buffer_at(buf, 0) >= '0' && buffer_at(buf, 0) <= '9'))
        return parse_number(item, buf);
    if (buffer_at(buf, 0) == '[') return parse_array(item, buf);
    if (buffer_at(buf, 0) == '{') return parse_object(item, buf);
    return 0;
}

cJSON *cJSON_Parse(const char *value) {
    if (!value) return NULL;
    parse_buffer buf = { value, 0, strlen(value) + 1 };
    cJSON *item = cJSON_New_Item();
    if (!item) return NULL;
    if (!parse_value(item, &buf)) { cJSON_Delete(item); return NULL; }
    return item;
}

/* Accessors */
cJSON *cJSON_GetObjectItemCaseSensitive(const cJSON * const object, const char * const string) {
    if (!object || !string) return NULL;
    cJSON *cur = object->child;
    while (cur) {
        if (cur->string && strcmp(cur->string, string) == 0) return cur;
        cur = cur->next;
    }
    return NULL;
}

cJSON *cJSON_GetArrayItem(const cJSON *array, int index) {
    if (!array) return NULL;
    cJSON *cur = array->child;
    while (cur && index > 0) { cur = cur->next; index--; }
    return (index == 0) ? cur : NULL;
}

int cJSON_GetArraySize(const cJSON *array) {
    if (!array) return 0;
    cJSON *cur = array->child;
    int size = 0;
    while (cur) { size++; cur = cur->next; }
    return size;
}

int cJSON_IsNumber(const cJSON * const item) { return item && (item->type & 0xFF) == cJSON_Number; }
int cJSON_IsString(const cJSON * const item) { return item && (item->type & 0xFF) == cJSON_String; }
int cJSON_IsArray(const cJSON * const item) { return item && (item->type & 0xFF) == cJSON_Array; }
int cJSON_IsObject(const cJSON * const item) { return item && (item->type & 0xFF) == cJSON_Object; }
int cJSON_IsBool(const cJSON * const item) { return item && ((item->type & 0xFF) == cJSON_True || (item->type & 0xFF) == cJSON_False); }
int cJSON_IsTrue(const cJSON * const item) { return item && (item->type & 0xFF) == cJSON_True; }
