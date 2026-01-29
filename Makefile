EXTENSION = pg_viewctl
MODULE_big = pg_viewctl
DATA = pg_viewctl--0.1.0.sql
OBJS = pg_viewctl.o

PG_CONFIG ?= pg_config
PGXS := $(shell $(PG_CONFIG) --pgxs)
include $(PGXS)
