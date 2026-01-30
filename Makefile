EXTENSION = pg_viewctl
MODULE_big = pg_viewctl
DATA = pg_viewctl--0.1.0.sql
OBJS = pg_viewctl.o

SQL_QUERIES = $(wildcard sql_queries/*.sql)
GENERATED_H = sql_queries.h

PG_CONFIG ?= pg_config
PGXS := $(shell $(PG_CONFIG) --pgxs)
include $(PGXS)

$(GENERATED_H): $(SQL_QUERIES) gen_sql_headers.sh
	./gen_sql_headers.sh sql_queries $@

pg_viewctl.o: $(GENERATED_H)

clean: clean-generated
clean-generated:
	rm -f $(GENERATED_H)
