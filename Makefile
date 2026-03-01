EXTENSION = pg_viewctl
MODULE_big = pg_viewctl
DATA = pg_viewctl--0.1.0.sql
OBJS = pg_viewctl.o
REGRESS = pg_viewctl_deps pg_viewctl_deprecation

SQL_QUERIES = $(wildcard sql_queries/*.sql)
GENERATED_H = sql_queries.h

PG_CONFIG ?= pg_config
PGXS := $(shell $(PG_CONFIG) --pgxs)
include $(PGXS)
