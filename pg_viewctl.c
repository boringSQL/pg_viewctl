#include "postgres.h"

#include "executor/spi.h"
#include "fmgr.h"
#include "funcapi.h"
#include "utils/builtins.h"

#include "sql_queries.h"

PG_MODULE_MAGIC;

#define COLDEPS_COLS 5

typedef struct {
	int total_rows;
	int current_row;
	char **values;  /* flat array: total_rows * COLDEPS_COLS strings */
} ColumnDepsCtx;

static TupleDesc
build_coldeps_tupdesc(void) {
	TupleDesc tupdesc = CreateTemplateTupleDesc(COLDEPS_COLS);
	TupleDescInitEntry(tupdesc, 1, "dependent_schema", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 2, "dependent_view", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 3, "dependent_column", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 4, "source_column", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 5, "dependency_type", TEXTOID, -1, 0);
	return BlessTupleDesc(tupdesc);
}

/* --- get_column_dependencies --- */

PG_FUNCTION_INFO_V1(get_column_dependencies);

Datum
get_column_dependencies(PG_FUNCTION_ARGS) {
	FuncCallContext *funcctx;
	ColumnDepsCtx *ctx;

	if (SRF_IS_FIRSTCALL()) {
		MemoryContext oldctx;
		Oid argtypes[2] = {TEXTOID, TEXTOID};
		Datum argvals[2];
		int ret;
		uint64 nrows;

		funcctx = SRF_FIRSTCALL_INIT();
		oldctx = MemoryContextSwitchTo(funcctx->multi_call_memory_ctx);

		funcctx->tuple_desc = build_coldeps_tupdesc();

		ctx = palloc0(sizeof(ColumnDepsCtx));
		funcctx->user_fctx = ctx;

		argvals[0] = PG_GETARG_DATUM(0);
		argvals[1] = PG_GETARG_DATUM(1);

		SPI_connect();
		ret = SPI_execute_with_args(sql_get_column_deps,
									2, argtypes, argvals, NULL, true, 0);

		if (ret != SPI_OK_SELECT) {
			SPI_finish();
			MemoryContextSwitchTo(oldctx);
			SRF_RETURN_DONE(funcctx);
		}

		nrows = SPI_processed;
		ctx->total_rows = (int) nrows;
		ctx->current_row = 0;

		if (nrows > 0) {
			uint64 i;

			ctx->values = palloc(sizeof(char *) * nrows * COLDEPS_COLS);
			for (i = 0; i < nrows; i++) {
				HeapTuple spi_tuple = SPI_tuptable->vals[i];
				TupleDesc spi_tupdesc = SPI_tuptable->tupdesc;
				int base = i * COLDEPS_COLS;
				int col;

				for (col = 1; col <= COLDEPS_COLS; col++)
					ctx->values[base + col - 1] =
						SPI_getvalue(spi_tuple, spi_tupdesc, col);
			}
		}

		SPI_finish();
		MemoryContextSwitchTo(oldctx);
	}

	funcctx = SRF_PERCALL_SETUP();
	ctx = funcctx->user_fctx;

	if (ctx->current_row < ctx->total_rows) {
		Datum vals[COLDEPS_COLS];
		bool nulls[COLDEPS_COLS] = {false};
		int base = ctx->current_row * COLDEPS_COLS;
		HeapTuple tuple;
		int i;

		for (i = 0; i < COLDEPS_COLS; i++) {
			if (ctx->values[base + i])
				vals[i] = CStringGetTextDatum(ctx->values[base + i]);
			else {
				vals[i] = (Datum) 0;
				nulls[i] = true;
			}
		}

		tuple = heap_form_tuple(funcctx->tuple_desc, vals, nulls);
		ctx->current_row++;
		SRF_RETURN_NEXT(funcctx, HeapTupleGetDatum(tuple));
	}

	SRF_RETURN_DONE(funcctx);
}
