PREFIX  ?= /usr/local
BINDIR  := $(PREFIX)/bin
TARGET  := target/release/breadshot

.PHONY: build install uninstall

build:
	cargo build --release

install: build
	install -Dm755 $(TARGET) $(DESTDIR)$(BINDIR)/breadshot

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/breadshot
