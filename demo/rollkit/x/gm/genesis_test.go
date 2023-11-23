package gm_test

import (
	"testing"

	"github.com/stretchr/testify/require"
	keepertest "gm/testutil/keeper"
	"gm/testutil/nullify"
	"gm/x/gm"
	"gm/x/gm/types"
)

func TestGenesis(t *testing.T) {
	genesisState := types.GenesisState{
		Params: types.DefaultParams(),

		// this line is used by starport scaffolding # genesis/test/state
	}

	k, ctx := keepertest.GmKeeper(t)
	gm.InitGenesis(ctx, *k, genesisState)
	got := gm.ExportGenesis(ctx, *k)
	require.NotNil(t, got)

	nullify.Fill(&genesisState)
	nullify.Fill(got)

	// this line is used by starport scaffolding # genesis/test/assert
}
