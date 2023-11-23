package keeper_test

import (
	"testing"

	"github.com/stretchr/testify/require"
	testkeeper "gm/testutil/keeper"
	"gm/x/gm/types"
)

func TestGetParams(t *testing.T) {
	k, ctx := testkeeper.GmKeeper(t)
	params := types.DefaultParams()

	k.SetParams(ctx, params)

	require.EqualValues(t, params, k.GetParams(ctx))
}
