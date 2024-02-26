package types

const (
	// ModuleName defines the module name
	ModuleName = "gm"

	// StoreKey defines the primary module store key
	StoreKey = ModuleName

	// MemStoreKey defines the in-memory store key
	MemStoreKey = "mem_gm"
)

var (
	ParamsKey = []byte("p_gm")
)

func KeyPrefix(p string) []byte {
	return []byte(p)
}
