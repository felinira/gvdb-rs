CFLAGS = -c -Wall $(shell pkg-config --cflags gio-2.0)
LDFLAGS = $(shell pkg-config --libs gio-2.0)

SOURCES = $(wildcard *.c)
OBJECTS = $(SOURCES:.c=.o)
BIN = create-test-files

all: $(SOURCES) $(BIN)

$(BIN): $(OBJECTS)
	gcc $(OBJECTS) -o $@ $(LDFLAGS)

.c:
	gcc $(CFLAGS) $< -o $@

clean:
	rm -f $(OBJECTS) $(BIN)
