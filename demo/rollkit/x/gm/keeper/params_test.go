package keeper_test

import (
	"testing"

	"github.com/stretchr/testify/require"

	keepertest "gm/testutil/keeper"
	"gm/x/gm/types"
)

func TestGetParams(t *testing.T) {
	k, ctx := keepertest.GmKeeper(t)
	params := types.DefaultParams()

	require.NoError(t, k.SetParams(ctx, params))
	require.EqualValues(t, params, k.GetParams(ctx))
}
