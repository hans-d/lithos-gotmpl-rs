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
	"io"
	"os"
	"path/filepath"
	"reflect"
	"strconv"
	"strings"
	texttmpl "text/template"

	// Blank import ensures the CLI keeps a direct dependency on golang.org/x/crypto
	// so security updates remain pinned via go.mod.
	_ "golang.org/x/crypto/blake2b"
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

func wrapExtremumFunctions(funcs map[string]interface{}) {
	wrapIntExtremum(funcs, "min")
	wrapIntExtremum(funcs, "max")
	wrapFloatExtremum(funcs, "minf")
	wrapFloatExtremum(funcs, "maxf")
}

func wrapIntExtremum(funcs map[string]interface{}, name string) {
	original, ok := funcs[name].(func(interface{}, ...interface{}) int64)
	if !ok {
		return
	}
	funcs[name] = func(first interface{}, rest ...interface{}) int64 {
		flattened := flattenExtremumArgs(append([]interface{}{first}, rest...))
		if len(flattened) == 0 {
			return original(first, rest...)
		}
		return original(flattened[0], flattened[1:]...)
	}
}

func wrapFloatExtremum(funcs map[string]interface{}, name string) {
	original, ok := funcs[name].(func(interface{}, ...interface{}) float64)
	if !ok {
		return
	}
	funcs[name] = func(first interface{}, rest ...interface{}) float64 {
		flattened := flattenExtremumArgs(append([]interface{}{first}, rest...))
		if len(flattened) == 0 {
			return original(first, rest...)
		}
		return original(flattened[0], flattened[1:]...)
	}
}

func flattenExtremumArgs(args []interface{}) []interface{} {
	flattened := make([]interface{}, 0, len(args))
	for _, arg := range args {
		if arg == nil {
			flattened = append(flattened, arg)
			continue
		}
		val := reflect.ValueOf(arg)
		switch val.Kind() {
		case reflect.Slice, reflect.Array:
			// Treat byte slices as atomic values.
			if val.Type().Elem().Kind() == reflect.Uint8 {
				flattened = append(flattened, arg)
				continue
			}
			for i := 0; i < val.Len(); i++ {
				flattened = append(flattened, val.Index(i).Interface())
			}
		default:
			flattened = append(flattened, arg)
		}
	}
	return flattened
}

func main() {
	defaultCases := filepath.Join("..", "test-cases", "lithos-sprig.json")
	casesPath := flag.String("cases", defaultCases, "path to JSON file with function cases")
	includeSprig := flag.Bool("sprig", true, "include Sprig helper functions")
	flag.Parse()

	if err := run(os.Stdout, *casesPath, *includeSprig); err != nil {
		fail(err)
	}
}

func run(output io.Writer, casesPath string, includeSprig bool) error {
	cases, err := loadCases(casesPath)
	if err != nil {
		return err
	}

    var funcs map[string]interface{}
    if includeSprig {
        funcs = sprig.GenericFuncMap()
        wrapExtremumFunctions(funcs)
        funcs["splitn"] = func(sep, text string, n int) []string {
            return strings.SplitN(text, sep, n)
        }
    }
	results := make([]result, 0, len(cases))
	for _, c := range cases {
		res, errs := evaluateCase(funcs, includeSprig, c)
		if errMsg := collectErrors(errs); errMsg != "" {
			res.Error = errMsg
		}
		results = append(results, res)
	}

	encoder := json.NewEncoder(output)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(results); err != nil {
		return fmt.Errorf("encode results: %w", err)
	}
	return nil
}

func evaluateCase(funcs map[string]interface{}, includeSprig bool, c testCase) (result, []error) {
	res := result{
		Name:     c.Name,
		Function: c.Function,
		Args:     c.Args,
		Template: c.Template,
		Data:     c.Data,
		Expected: c.Expected,
	}

	var errs []error

	if c.Function != "" {
		if funcs == nil {
			errs = append(errs, fmt.Errorf("function %q requested but Sprig helpers are disabled", c.Function))
		} else {
			out, err := evaluate(funcs, c.Function, c.Args)
			if err != nil {
				errs = append(errs, err)
			} else {
				res.Output = out
			}
		}
	}

	if c.Template != "" {
		rendered, err := renderTemplate(c.Template, c.Data, includeSprig)
		if err != nil {
			errs = append(errs, err)
		} else {
			renderedCopy := rendered
			res.Rendered = &renderedCopy
			if c.Expected != nil && rendered != *c.Expected {
				errs = append(errs, fmt.Errorf("template output %q does not match expected %q", rendered, *c.Expected))
			}
		}
	}

	return res, errs
}

func collectErrors(errs []error) string {
	if len(errs) == 0 {
		return ""
	}

	messages := make([]string, len(errs))
	for i, err := range errs {
		messages[i] = err.Error()
	}
	return strings.Join(messages, "; ")
}

// loadCases reads the JSON file containing lithos-sprig test vectors.
func loadCases(path string) ([]testCase, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, fmt.Errorf("open cases file: %w", err)
	}
	defer func() {
		if cerr := file.Close(); cerr != nil {
			fmt.Fprintf(os.Stderr, "warning: closing cases file failed: %v\n", cerr)
		}
	}()

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

func renderTemplate(tpl string, data interface{}, includeSprig bool) (string, error) {
	tmpl := texttmpl.New("case")
	if includeSprig {
		funcs := sprig.TxtFuncMap()
		funcs["splitn"] = func(sep, text string, n int) []string {
			return strings.SplitN(text, sep, n)
		}
		tmpl = tmpl.Funcs(funcs)
	}

	parsed, err := tmpl.Parse(tpl)
	if err != nil {
		return "", fmt.Errorf("parse template: %w", err)
	}

	var buf bytes.Buffer
	if err := parsed.Execute(&buf, data); err != nil {
		return "", fmt.Errorf("execute template: %w", err)
	}
	return buf.String(), nil
}

func prepareArgs(args []interface{}, fnType reflect.Type) ([]reflect.Value, error) {
	prepared := make([]reflect.Value, len(args))
	for i, arg := range args {
		targetType := targetArgumentType(fnType, i)
		val, err := coerceArgument(arg, targetType)
		if err != nil {
			return nil, fmt.Errorf("argument %d: %w", i+1, err)
		}
		prepared[i] = val
	}
	return prepared, nil
}

func targetArgumentType(fnType reflect.Type, index int) reflect.Type {
	targetIndex := index
	if fnType.IsVariadic() && index >= fnType.NumIn()-1 {
		targetIndex = fnType.NumIn() - 1
	}

	targetType := fnType.In(targetIndex)
	if fnType.IsVariadic() && index >= fnType.NumIn()-1 {
		targetType = targetType.Elem()
	}
	return targetType
}

type coercionStrategy func(arg interface{}, targetType reflect.Type) (reflect.Value, bool, error)

var strategies = []coercionStrategy{
	coerceNilArg,
	coerceInterfaceArg,
	coerceNumberArg,
	coercePrimitiveArg,
}

func coerceArgument(arg interface{}, targetType reflect.Type) (reflect.Value, error) {
	for _, strategy := range strategies {
		if val, handled, err := strategy(arg, targetType); handled {
			if err != nil {
				return reflect.Value{}, err
			}
			return val, nil
		} else if err != nil {
			return reflect.Value{}, err
		}
	}
	return reflect.Value{}, fmt.Errorf("cannot coerce %T into %s", arg, targetType.String())
}

func coerceNilArg(arg interface{}, targetType reflect.Type) (reflect.Value, bool, error) {
	if arg != nil {
		return reflect.Value{}, false, nil
	}
	return zero(targetType), true, nil
}

func coerceInterfaceArg(arg interface{}, targetType reflect.Type) (reflect.Value, bool, error) {
	if targetType.Kind() != reflect.Interface {
		return reflect.Value{}, false, nil
	}
	return reflect.ValueOf(arg), true, nil
}

func coerceNumberArg(arg interface{}, targetType reflect.Type) (reflect.Value, bool, error) {
	switch v := arg.(type) {
	case json.Number:
		val, err := convertNumber(v, targetType)
		return val, true, err
	case float64:
		val, err := convertFloat64(v, targetType)
		return val, true, err
	}
	return reflect.Value{}, false, nil
}

func coercePrimitiveArg(arg interface{}, targetType reflect.Type) (reflect.Value, bool, error) {
	original := reflect.ValueOf(arg)
	if !original.IsValid() {
		return reflect.Value{}, false, nil
	}
	if val, ok := convertPrimitive(original, targetType); ok {
		return val, true, nil
	}
	return reflect.Value{}, false, nil
}

func convertPrimitive(original reflect.Value, targetType reflect.Type) (reflect.Value, bool) {
	if original.Type().AssignableTo(targetType) {
		return original, true
	}
	if original.Type().ConvertibleTo(targetType) {
		return original.Convert(targetType), true
	}
	return reflect.Value{}, false
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
