#include "postgres.h"

#include "executor/spi.h"
#include "fmgr.h"
#include "funcapi.h"
#include "utils/builtins.h"

#include "sql_queries.h"

PG_MODULE_MAGIC;

/* generic context for SRFs that materialize SPI results as text */
#define SRF_MAX_COLS 16

typedef struct {
	int ncols;
	int total_rows;
	int current_row;
	char **values;  /* flat array: total_rows * ncols strings */
} SRFCtx;

static HeapTuple
srf_emit_row(SRFCtx *ctx, TupleDesc tupdesc) {
	Datum vals[SRF_MAX_COLS];
	bool nulls[SRF_MAX_COLS];
	int base = ctx->current_row * ctx->ncols;
	int i;

	Assert(ctx->ncols <= SRF_MAX_COLS);
	memset(nulls, 0, sizeof(nulls));

	for (i = 0; i < ctx->ncols; i++) {
		if (ctx->values[base + i])
			vals[i] = CStringGetTextDatum(ctx->values[base + i]);
		else {
			vals[i] = (Datum) 0;
			nulls[i] = true;
		}
	}

	return heap_form_tuple(tupdesc, vals, nulls);
}

/* --- get_column_dependencies --- */

static TupleDesc
build_coldeps_tupdesc(void) {
	TupleDesc tupdesc = CreateTemplateTupleDesc(5);
	TupleDescInitEntry(tupdesc, 1, "dependent_schema", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 2, "dependent_view", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 3, "dependent_column", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 4, "source_column", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 5, "dependency_type", TEXTOID, -1, 0);
	return BlessTupleDesc(tupdesc);
}

PG_FUNCTION_INFO_V1(get_column_dependencies);

Datum
get_column_dependencies(PG_FUNCTION_ARGS) {
	FuncCallContext *funcctx;
	SRFCtx *ctx;

	if (SRF_IS_FIRSTCALL()) {
		MemoryContext oldctx;
		Oid argtypes[2] = {TEXTOID, TEXTOID};
		Datum argvals[2];
		int ret;
		uint64 nrows;

		funcctx = SRF_FIRSTCALL_INIT();
		oldctx = MemoryContextSwitchTo(funcctx->multi_call_memory_ctx);

		funcctx->tuple_desc = build_coldeps_tupdesc();

		ctx = palloc0(sizeof(SRFCtx));
		ctx->ncols = 5;
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

		if (nrows > 0) {
			uint64 i;

			ctx->values = palloc(sizeof(char *) * nrows * ctx->ncols);
			for (i = 0; i < nrows; i++) {
				HeapTuple spi_tuple = SPI_tuptable->vals[i];
				TupleDesc spi_tupdesc = SPI_tuptable->tupdesc;
				int base = i * ctx->ncols;
				int col;

				for (col = 1; col <= ctx->ncols; col++)
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
		HeapTuple tuple = srf_emit_row(ctx, funcctx->tuple_desc);
		ctx->current_row++;
		SRF_RETURN_NEXT(funcctx, HeapTupleGetDatum(tuple));
	}
	SRF_RETURN_DONE(funcctx);
}

/* --- analyze_drop_column --- */

static TupleDesc
build_analyze_drop_tupdesc(void) {
	TupleDesc tupdesc = CreateTemplateTupleDesc(5);
	TupleDescInitEntry(tupdesc, 1, "dependent_view", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 2, "dependent_column", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 3, "usage_type", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 4, "impact_severity", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 5, "usage_location", TEXTOID, -1, 0);
	return BlessTupleDesc(tupdesc);
}

PG_FUNCTION_INFO_V1(analyze_drop_column);

Datum
analyze_drop_column(PG_FUNCTION_ARGS) {
	FuncCallContext *funcctx;
	SRFCtx *ctx;

	if (SRF_IS_FIRSTCALL()) {
		MemoryContext oldctx;
		Oid argtypes[3] = {TEXTOID, TEXTOID, TEXTOID};
		Datum argvals[3];
		int ret;
		uint64 nrows;

		funcctx = SRF_FIRSTCALL_INIT();
		oldctx = MemoryContextSwitchTo(funcctx->multi_call_memory_ctx);

		funcctx->tuple_desc = build_analyze_drop_tupdesc();

		ctx = palloc0(sizeof(SRFCtx));
		ctx->ncols = 5;
		funcctx->user_fctx = ctx;

		argvals[0] = PG_GETARG_DATUM(0);
		argvals[1] = PG_GETARG_DATUM(1);
		argvals[2] = PG_GETARG_DATUM(2);

		SPI_connect();
		ret = SPI_execute_with_args(sql_analyze_drop_column,
									3, argtypes, argvals, NULL, true, 0);

		if (ret != SPI_OK_SELECT) {
			SPI_finish();
			MemoryContextSwitchTo(oldctx);
			SRF_RETURN_DONE(funcctx);
		}

		nrows = SPI_processed;
		ctx->total_rows = (int) nrows;

		if (nrows > 0) {
			uint64 i;

			ctx->values = palloc(sizeof(char *) * nrows * ctx->ncols);
			for (i = 0; i < nrows; i++) {
				HeapTuple spi_tuple = SPI_tuptable->vals[i];
				TupleDesc spi_tupdesc = SPI_tuptable->tupdesc;
				int base = i * ctx->ncols;
				int col;

				for (col = 1; col <= ctx->ncols; col++)
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
		HeapTuple tuple = srf_emit_row(ctx, funcctx->tuple_desc);
		ctx->current_row++;
		SRF_RETURN_NEXT(funcctx, HeapTupleGetDatum(tuple));
	}
	SRF_RETURN_DONE(funcctx);
}

/* --- deprecate_column --- */

PG_FUNCTION_INFO_V1(deprecate_column);

Datum
deprecate_column(PG_FUNCTION_ARGS) {
	Oid argtypes_val[3] = {TEXTOID, TEXTOID, TEXTOID};
	Datum argvals_val[3];
	Oid argtypes_ups[5] = {TEXTOID, TEXTOID, TEXTOID, TEXTOID, DATEOID};
	Datum argvals_ups[5];
	char argnulls[5];
	char *schema_name, *view_name, *column_name;
	StringInfoData msg;
	int ret;

	if (PG_ARGISNULL(0) || PG_ARGISNULL(1) || PG_ARGISNULL(2))
		ereport(ERROR,
				(errcode(ERRCODE_NULL_VALUE_NOT_ALLOWED),
				 errmsg("schema_name, view_name, and column_name must not be NULL")));

	schema_name = text_to_cstring(PG_GETARG_TEXT_PP(0));
	view_name   = text_to_cstring(PG_GETARG_TEXT_PP(1));
	column_name = text_to_cstring(PG_GETARG_TEXT_PP(2));

	SPI_connect();

	/* validate column exists on the view */
	argvals_val[0] = PG_GETARG_DATUM(0);
	argvals_val[1] = PG_GETARG_DATUM(1);
	argvals_val[2] = PG_GETARG_DATUM(2);

	ret = SPI_execute_with_args(sql_validate_column_exists,
								3, argtypes_val, argvals_val, NULL, true, 1);

	if (ret != SPI_OK_SELECT || SPI_processed == 0) {
		SPI_finish();
		ereport(ERROR,
				(errcode(ERRCODE_UNDEFINED_COLUMN),
				 errmsg("column \"%s\" not found on view \"%s.%s\"",
						column_name, schema_name, view_name)));
	}

	/* upsert into pgvc_deprecated_columns */
	argvals_ups[0] = PG_GETARG_DATUM(0);
	argvals_ups[1] = PG_GETARG_DATUM(1);
	argvals_ups[2] = PG_GETARG_DATUM(2);

	argnulls[0] = ' ';
	argnulls[1] = ' ';
	argnulls[2] = ' ';

	if (PG_ARGISNULL(3)) {
		argnulls[3] = 'n';
		argvals_ups[3] = (Datum) 0;
	} else {
		argnulls[3] = ' ';
		argvals_ups[3] = PG_GETARG_DATUM(3);
	}

	if (PG_ARGISNULL(4)) {
		argnulls[4] = 'n';
		argvals_ups[4] = (Datum) 0;
	} else {
		argnulls[4] = ' ';
		argvals_ups[4] = PG_GETARG_DATUM(4);
	}

	ret = SPI_execute_with_args(sql_deprecate_column_upsert,
								5, argtypes_ups, argvals_ups, argnulls, false, 0);

	SPI_finish();

	if (ret != SPI_OK_INSERT)
		ereport(ERROR,
				(errcode(ERRCODE_INTERNAL_ERROR),
				 errmsg("failed to deprecate column \"%s.%s.%s\"",
						schema_name, view_name, column_name)));

	initStringInfo(&msg);
	appendStringInfo(&msg, "column %s.%s.%s deprecated",
					 schema_name, view_name, column_name);
	PG_RETURN_TEXT_P(cstring_to_text(msg.data));
}

/* --- undeprecate_column --- */

PG_FUNCTION_INFO_V1(undeprecate_column);

Datum
undeprecate_column(PG_FUNCTION_ARGS) {
	char *schema_name = text_to_cstring(PG_GETARG_TEXT_PP(0));
	char *view_name   = text_to_cstring(PG_GETARG_TEXT_PP(1));
	char *column_name = text_to_cstring(PG_GETARG_TEXT_PP(2));
	Oid argtypes[3] = {TEXTOID, TEXTOID, TEXTOID};
	Datum argvals[3];
	StringInfoData msg;
	int ret;

	SPI_connect();

	argvals[0] = PG_GETARG_DATUM(0);
	argvals[1] = PG_GETARG_DATUM(1);
	argvals[2] = PG_GETARG_DATUM(2);

	ret = SPI_execute_with_args(sql_undeprecate_column,
								3, argtypes, argvals, NULL, false, 0);

	SPI_finish();

	if (ret != SPI_OK_DELETE)
		ereport(ERROR,
				(errcode(ERRCODE_INTERNAL_ERROR),
				 errmsg("failed to undeprecate column \"%s.%s.%s\"",
						schema_name, view_name, column_name)));

	initStringInfo(&msg);
	if (SPI_processed > 0)
		appendStringInfo(&msg, "column %s.%s.%s undeprecated",
						 schema_name, view_name, column_name);
	else
		appendStringInfo(&msg, "column %s.%s.%s was not marked as deprecated",
						 schema_name, view_name, column_name);

	PG_RETURN_TEXT_P(cstring_to_text(msg.data));
}

/* --- get_deprecated_columns --- */

static TupleDesc
build_deprecated_cols_tupdesc(void) {
	TupleDesc tupdesc = CreateTemplateTupleDesc(6);
	TupleDescInitEntry(tupdesc, 1, "schema_name", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 2, "view_name", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 3, "column_name", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 4, "deprecation_message", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 5, "removal_date", TEXTOID, -1, 0);
	TupleDescInitEntry(tupdesc, 6, "deprecated_at", TEXTOID, -1, 0);
	return BlessTupleDesc(tupdesc);
}

PG_FUNCTION_INFO_V1(get_deprecated_columns);

Datum
get_deprecated_columns(PG_FUNCTION_ARGS) {
	FuncCallContext *funcctx;
	SRFCtx *ctx;

	if (SRF_IS_FIRSTCALL()) {
		MemoryContext oldctx;
		Oid argtypes[1] = {TEXTOID};
		Datum argvals[1];
		char argnulls[1];
		int ret;
		uint64 nrows;

		funcctx = SRF_FIRSTCALL_INIT();
		oldctx = MemoryContextSwitchTo(funcctx->multi_call_memory_ctx);

		funcctx->tuple_desc = build_deprecated_cols_tupdesc();

		ctx = palloc0(sizeof(SRFCtx));
		ctx->ncols = 6;
		funcctx->user_fctx = ctx;

		if (PG_ARGISNULL(0)) {
			argnulls[0] = 'n';
			argvals[0] = (Datum) 0;
		} else {
			argnulls[0] = ' ';
			argvals[0] = PG_GETARG_DATUM(0);
		}

		SPI_connect();
		ret = SPI_execute_with_args(sql_get_deprecated_columns,
									1, argtypes, argvals, argnulls, true, 0);

		if (ret != SPI_OK_SELECT) {
			SPI_finish();
			MemoryContextSwitchTo(oldctx);
			SRF_RETURN_DONE(funcctx);
		}

		nrows = SPI_processed;
		ctx->total_rows = (int) nrows;

		if (nrows > 0) {
			uint64 i;

			ctx->values = palloc(sizeof(char *) * nrows * ctx->ncols);
			for (i = 0; i < nrows; i++) {
				HeapTuple spi_tuple = SPI_tuptable->vals[i];
				TupleDesc spi_tupdesc = SPI_tuptable->tupdesc;
				int base = i * ctx->ncols;
				int col;

				for (col = 1; col <= ctx->ncols; col++)
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
		HeapTuple tuple = srf_emit_row(ctx, funcctx->tuple_desc);
		ctx->current_row++;
		SRF_RETURN_NEXT(funcctx, HeapTupleGetDatum(tuple));
	}
	SRF_RETURN_DONE(funcctx);
}

/* --- check_column_deprecated --- */

PG_FUNCTION_INFO_V1(check_column_deprecated);

Datum
check_column_deprecated(PG_FUNCTION_ARGS) {
	Oid argtypes[3] = {TEXTOID, TEXTOID, TEXTOID};
	Datum argvals[3];
	int ret;

	SPI_connect();

	argvals[0] = PG_GETARG_DATUM(0);
	argvals[1] = PG_GETARG_DATUM(1);
	argvals[2] = PG_GETARG_DATUM(2);

	ret = SPI_execute_with_args(sql_check_column_deprecated,
								3, argtypes, argvals, NULL, true, 1);

	if (ret != SPI_OK_SELECT || SPI_processed == 0) {
		SPI_finish();
		PG_RETURN_NULL();
	}

	{
		HeapTuple spi_tuple = SPI_tuptable->vals[0];
		TupleDesc spi_tupdesc = SPI_tuptable->tupdesc;
		char *message = SPI_getvalue(spi_tuple, spi_tupdesc, 1);
		char *removal = SPI_getvalue(spi_tuple, spi_tupdesc, 2);
		char *schema  = text_to_cstring(PG_GETARG_TEXT_PP(0));
		char *view    = text_to_cstring(PG_GETARG_TEXT_PP(1));
		char *column  = text_to_cstring(PG_GETARG_TEXT_PP(2));
		StringInfoData msg;

		initStringInfo(&msg);
		appendStringInfo(&msg, "WARNING: column %s.%s.%s is deprecated",
						 schema, view, column);
		if (message)
			appendStringInfo(&msg, " — %s", message);
		if (removal)
			appendStringInfo(&msg, " (removal: %s)", removal);

		SPI_finish();
		PG_RETURN_TEXT_P(cstring_to_text(msg.data));
	}
}
