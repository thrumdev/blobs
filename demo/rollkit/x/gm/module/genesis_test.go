package gm_test

import (
	"testing"

	keepertest "gm/testutil/keeper"
	"gm/testutil/nullify"
	gm "gm/x/gm/module"
	"gm/x/gm/types"

	"github.com/stretchr/testify/require"
)

func TestGenesis(t *testing.T) {
	genesisState := types.GenesisState{
		Params: types.DefaultParams(),

		// this line is used by starport scaffolding # genesis/test/state
	}

	k, ctx := keepertest.GmKeeper(t)
	gm.InitGenesis(ctx, k, genesisState)
	got := gm.ExportGenesis(ctx, k)
	require.NotNil(t, got)

	nullify.Fill(&genesisState)
	nullify.Fill(got)

	// this line is used by starport scaffolding # genesis/test/assert
}
