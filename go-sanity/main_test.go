package main

import (
	"encoding/json"
	"reflect"
	"testing"
)

func TestCoerceNumberArgJSONInt(t *testing.T) {
	target := reflect.TypeOf(int(0))
	val, handled, err := coerceNumberArg(json.Number("42"), target)
	if !handled {
		t.Fatalf("expected strategy to handle json.Number input")
	}
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got := val.Int(); got != 42 {
		t.Fatalf("expected 42, got %d", got)
	}
}

func TestCoerceNumberArgJSONInvalidTarget(t *testing.T) {
	target := reflect.TypeOf(true)
	_, handled, err := coerceNumberArg(json.Number("1"), target)
	if !handled {
		t.Fatalf("expected strategy to handle json.Number input")
	}
	if err == nil {
		t.Fatalf("expected error for incompatible target type")
	}
}

func TestConvertPrimitiveAssignable(t *testing.T) {
	val, ok := convertPrimitive(reflect.ValueOf("hello"), reflect.TypeOf(""))
	if !ok {
		t.Fatalf("expected assignable value to succeed")
	}
	if val.Interface() != "hello" {
		t.Fatalf("unexpected value: %v", val.Interface())
	}
}

func TestConvertPrimitiveConvertible(t *testing.T) {
	val, ok := convertPrimitive(reflect.ValueOf(int32(7)), reflect.TypeOf(int64(0)))
	if !ok {
		t.Fatalf("expected convertible value to succeed")
	}
	if got := val.Int(); got != 7 {
		t.Fatalf("expected 7, got %d", got)
	}
}

func TestConvertPrimitiveNotConvertible(t *testing.T) {
	if _, ok := convertPrimitive(reflect.ValueOf(struct{}{}), reflect.TypeOf("")); ok {
		t.Fatalf("expected conversion to fail for incompatible types")
	}
}
