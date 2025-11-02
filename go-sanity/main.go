// SPDX-License-Identifier: Apache-2.0 OR MIT
// Package main provides a small CLI that replays the lithos-sprig test cases
// against Go's text/template implementation and the sprig helper library. The
// resulting JSON output is consumed by the Rust parity tests.
package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strconv"
	"strings"
	texttmpl "text/template"

	"github.com/Masterminds/sprig/v3"
)

var (
	errorType = reflect.TypeOf((*error)(nil)).Elem()
)

type testCase struct {
	Name     string        `json:"name"`
	Function string        `json:"function"`
	Args     []interface{} `json:"args"`
	Template string        `json:"template"`
	Data     interface{}   `json:"data"`
	Expected *string       `json:"expected"`
}

type result struct {
	Name     string        `json:"name,omitempty"`
	Function string        `json:"function,omitempty"`
	Args     []interface{} `json:"args,omitempty"`
	Output   interface{}   `json:"output,omitempty"`
	Template string        `json:"template,omitempty"`
	Data     interface{}   `json:"data,omitempty"`
	Rendered *string       `json:"rendered,omitempty"`
	Expected *string       `json:"expected,omitempty"`
	Error    string        `json:"error,omitempty"`
}

func main() {
	defaultCases := filepath.Join("..", "test-cases", "lithos-sprig.json")
	casesPath := flag.String("cases", defaultCases, "path to JSON file with function cases")
	flag.Parse()

	cases, err := loadCases(*casesPath)
	if err != nil {
		fail(err)
	}

	funcs := sprig.GenericFuncMap()
	funcs["splitn"] = func(sep, text string, n int) []string {
		return strings.SplitN(text, sep, n)
	}
	results := make([]result, 0, len(cases))
	for _, c := range cases {
		res := result{
			Name:     c.Name,
			Function: c.Function,
			Args:     c.Args,
			Template: c.Template,
			Data:     c.Data,
			Expected: c.Expected,
		}

		var errs []string

		if c.Function != "" {
			out, evalErr := evaluate(funcs, c.Function, c.Args)
			if evalErr != nil {
				errs = append(errs, evalErr.Error())
			} else {
				res.Output = out
			}
		}

		if c.Template != "" {
			rendered, err := renderTemplate(c.Template, c.Data)
			if err != nil {
				errs = append(errs, err.Error())
			} else {
				renderedCopy := rendered
				res.Rendered = &renderedCopy
				if c.Expected != nil && rendered != *c.Expected {
					errs = append(errs, fmt.Sprintf("template output %q does not match expected %q", rendered, *c.Expected))
				}
			}
		}

		if len(errs) > 0 {
			res.Error = strings.Join(errs, "; ")
		}

		results = append(results, res)
	}

	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(results); err != nil {
		fail(err)
	}
}

// loadCases reads the JSON file containing lithos-sprig test vectors.
func loadCases(path string) ([]testCase, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, fmt.Errorf("open cases file: %w", err)
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	decoder.UseNumber()

	var cases []testCase
	if err := decoder.Decode(&cases); err != nil {
		return nil, fmt.Errorf("decode cases: %w", err)
	}
	return cases, nil
}

func evaluate(funcs map[string]interface{}, name string, args []interface{}) (interface{}, error) {
	fn, ok := funcs[name]
	if !ok {
		return nil, fmt.Errorf("function %q not found in sprig map", name)
	}

	fnValue := reflect.ValueOf(fn)
	fnType := fnValue.Type()
	inCount := len(args)

	if fnType.IsVariadic() {
		minArgs := fnType.NumIn() - 1
		if inCount < minArgs {
			return nil, fmt.Errorf("function %q expects at least %d arguments, got %d", name, minArgs, inCount)
		}
	} else if inCount != fnType.NumIn() {
		return nil, fmt.Errorf("function %q expects %d arguments, got %d", name, fnType.NumIn(), inCount)
	}

	prepared, err := prepareArgs(args, fnType)
	if err != nil {
		return nil, fmt.Errorf("function %q: %w", name, err)
	}

	callResults := fnValue.Call(prepared)
	out, err := collectResults(callResults)
	if err != nil {
		return nil, err
	}
	return out, nil
}

func renderTemplate(tpl string, data interface{}) (string, error) {
	funcs := sprig.TxtFuncMap()
	funcs["splitn"] = func(sep, text string, n int) []string {
		return strings.SplitN(text, sep, n)
	}

	tmpl, err := texttmpl.New("case").Funcs(funcs).Parse(tpl)
	if err != nil {
		return "", fmt.Errorf("parse template: %w", err)
	}

	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, data); err != nil {
		return "", fmt.Errorf("execute template: %w", err)
	}
	return buf.String(), nil
}

func prepareArgs(args []interface{}, fnType reflect.Type) ([]reflect.Value, error) {
	prepared := make([]reflect.Value, len(args))
	for i, arg := range args {
		targetIndex := i
		if fnType.IsVariadic() && i >= fnType.NumIn()-1 {
			targetIndex = fnType.NumIn() - 1
		}

		targetType := fnType.In(targetIndex)
		if fnType.IsVariadic() && i >= fnType.NumIn()-1 {
			targetType = targetType.Elem()
		}

		val, err := coerce(arg, targetType)
		if err != nil {
			return nil, fmt.Errorf("argument %d: %w", i+1, err)
		}
		prepared[i] = val
	}
	return prepared, nil
}

func coerce(arg interface{}, targetType reflect.Type) (reflect.Value, error) {
	if arg == nil {
		return zero(targetType), nil
	}

	if targetType.Kind() == reflect.Interface {
		return reflect.ValueOf(arg), nil
	}

	switch v := arg.(type) {
	case json.Number:
		return convertNumber(v, targetType)
	case float64:
		return convertFloat64(v, targetType)
	}

	original := reflect.ValueOf(arg)
	if original.Type().AssignableTo(targetType) {
		return original, nil
	}
	if original.Type().ConvertibleTo(targetType) {
		return original.Convert(targetType), nil
	}

	return reflect.Value{}, fmt.Errorf("cannot coerce %T into %s", arg, targetType.String())
}

func convertNumber(num json.Number, targetType reflect.Type) (reflect.Value, error) {
	if i64, err := num.Int64(); err == nil {
		return convertInt64(i64, targetType)
	}
	if f64, err := num.Float64(); err == nil {
		return convertFloat64(f64, targetType)
	}
	return reflect.Value{}, fmt.Errorf("unable to parse number %q", num.String())
}

func convertFloat64(f float64, targetType reflect.Type) (reflect.Value, error) {
	switch targetType.Kind() {
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return convertInt64(int64(f), targetType)
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		return convertUint64(uint64(f), targetType)
	case reflect.Float32, reflect.Float64:
		val := reflect.New(targetType).Elem()
		val.SetFloat(f)
		return val, nil
	}
	return reflect.Value{}, fmt.Errorf("cannot convert float64 into %s", targetType.String())
}

func convertInt64(i int64, targetType reflect.Type) (reflect.Value, error) {
	switch targetType.Kind() {
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		val := reflect.New(targetType).Elem()
		val.SetInt(i)
		return val, nil
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		if i < 0 {
			return reflect.Value{}, fmt.Errorf("negative value %d cannot become %s", i, targetType.String())
		}
		return convertUint64(uint64(i), targetType)
	case reflect.Float32, reflect.Float64:
		val := reflect.New(targetType).Elem()
		val.SetFloat(float64(i))
		return val, nil
	case reflect.String:
		return reflect.ValueOf(strconv.FormatInt(i, 10)), nil
	}
	return reflect.Value{}, fmt.Errorf("cannot convert %d into %s", i, targetType.String())
}

func convertUint64(u uint64, targetType reflect.Type) (reflect.Value, error) {
	switch targetType.Kind() {
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		val := reflect.New(targetType).Elem()
		val.SetUint(u)
		return val, nil
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		val := reflect.New(targetType).Elem()
		val.SetInt(int64(u))
		return val, nil
	case reflect.Float32, reflect.Float64:
		val := reflect.New(targetType).Elem()
		val.SetFloat(float64(u))
		return val, nil
	case reflect.String:
		return reflect.ValueOf(strconv.FormatUint(u, 10)), nil
	}
	return reflect.Value{}, fmt.Errorf("cannot convert %d into %s", u, targetType.String())
}

func collectResults(callResults []reflect.Value) (interface{}, error) {
	if len(callResults) == 0 {
		return nil, nil
	}

	if last := callResults[len(callResults)-1]; last.Type().Implements(errorType) {
		if !last.IsNil() {
			return nil, last.Interface().(error)
		}
		callResults = callResults[:len(callResults)-1]
	}

	switch len(callResults) {
	case 0:
		return nil, nil
	case 1:
		return callResults[0].Interface(), nil
	default:
		out := make([]interface{}, len(callResults))
		for i, v := range callResults {
			out[i] = v.Interface()
		}
		return out, nil
	}
}

func zero(targetType reflect.Type) reflect.Value {
	switch targetType.Kind() {
	case reflect.Interface:
		return reflect.Zero(targetType)
	case reflect.String:
		return reflect.ValueOf("")
	default:
		return reflect.Zero(targetType)
	}
}

func fail(err error) {
	fmt.Fprintln(os.Stderr, err)
	os.Exit(1)
}
