mod tools 'tools.just'

default: tools::ensure-release-plz
    release-plz release --dry-run
