CC       ?= gcc
PKG      := gtk+-3.0 gtk-layer-shell-0
CFLAGS   := -O2 -Wall -Wextra $(shell pkg-config --cflags $(PKG))
LDFLAGS  := $(shell pkg-config --libs $(PKG))
PREFIX   ?= /usr/local

TARGET   := sliders_popup
SRCS     := main.c cJSON.c
OBJS     := $(SRCS:.c=.o)

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) -o $@ $^ $(LDFLAGS) -s

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

install: $(TARGET)
	install -Dm755 $(TARGET) $(DESTDIR)$(PREFIX)/bin/$(TARGET)

clean:
	rm -f $(OBJS) $(TARGET)

.PHONY: all install clean
