default:
    just --list

mod rust '../rust.just'

test: rust::test
