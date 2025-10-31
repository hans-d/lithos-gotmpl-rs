// SPDX-License-Identifier: Apache-2.0 OR MIT
package main

import (
	"fmt"
	"github.com/Masterminds/sprig/v3"
)

func main() {
	funcs := sprig.GenericFuncMap()
	keys := funcs["keys"].(func(...map[string]interface{}) []string)
	fmt.Println("keys (b,a map):", keys(map[string]interface{}{"b": 2, "a": 1}))
	fmt.Println("keys (a,b map):", keys(map[string]interface{}{"a": 1, "b": 2}))
}
