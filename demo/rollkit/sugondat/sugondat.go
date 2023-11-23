package sugondat

import (
	"context"
	"encoding/json"

	"github.com/filecoin-project/go-jsonrpc"
	ds "github.com/ipfs/go-datastore"
	"github.com/rollkit/rollkit/da"
	"github.com/rollkit/rollkit/da/registry"
	"github.com/rollkit/rollkit/third_party/log"
	"github.com/rollkit/rollkit/types"
)

type Blob struct {
	Data []byte `json:"data"`
}

type SugondatAPI struct {
	// Retrieves the blobs at the given height from the data availability layer at the given namespace.
	Retrieve func(string, uint64) ([]*Blob, error)
	// Submits the given blobs to the data availability layer at the given namespace.
	// Returns the height of the block that contains the blobs.
	Submit func(string, []*Blob) (uint64, error)
}

type RpcClient struct {
	closer jsonrpc.ClientCloser
	api    SugondatAPI
}

type DataAvailabilityLayerClient struct {
	logger log.Logger
	config Config
	rpc    RpcClient
}

var _ da.DataAvailabilityLayerClient = &DataAvailabilityLayerClient{}
var _ da.BlockRetriever = &DataAvailabilityLayerClient{}

type Config struct {
	BaseURL   string `json:"base_url"`
	Namespace string `json:"namespace"`
}

func Register() error {
	return registry.Register("sugondat", func() da.DataAvailabilityLayerClient {
		return &DataAvailabilityLayerClient{}
	})
}

func (c *DataAvailabilityLayerClient) Init(namespaceID types.NamespaceID, config []byte, kvStore ds.Datastore, logger log.Logger) error {
	c.logger = logger
	if len(config) > 0 {
		c.logger.Info("initializing Sugondat Data Availability Layer Client", "config", string(config))
		if err := json.Unmarshal(config, &c.config); err != nil {
			return err
		}
	}
	return nil
}

func (c *DataAvailabilityLayerClient) Start() error {
	c.logger.Info("starting Sugondat Data Availability Layer Client", "baseURL", c.config.BaseURL)
	closer, err := jsonrpc.NewClient(context.Background(), c.config.BaseURL, "Rollkit", &c.rpc.api, nil)
	if err != nil {
		return err
	}
	c.rpc.closer = closer
	return nil
}

func (c *DataAvailabilityLayerClient) Stop() error {
	c.logger.Info("stopping Sugondat Data Availability Layer Client")
	c.rpc.closer()
	return nil
}

// RetrieveBlocks gets a batch of blocks from DA layer.
func (c *DataAvailabilityLayerClient) RetrieveBlocks(ctx context.Context, dataLayerHeight uint64) da.ResultRetrieveBlocks {
	c.logger.Info("retrieving blocks from Sugondat Data Availability Layer", "dataLayerHeight", dataLayerHeight)
	blobs, err := c.rpc.api.Retrieve(c.config.Namespace, dataLayerHeight)
	if err != nil {
		return da.ResultRetrieveBlocks{
			BaseResult: da.BaseResult{
				Code:    da.StatusError,
				Message: err.Error(),
			},
		}
	}

	blocks := make([]*types.Block, len(blobs))
	for i, blob := range blobs {
		blocks[i] = &types.Block{}
		if err := blocks[i].UnmarshalBinary(blob.Data); err != nil {
			return da.ResultRetrieveBlocks{
				BaseResult: da.BaseResult{
					Code:    da.StatusError,
					Message: err.Error(),
				},
			}
		}
	}

	return da.ResultRetrieveBlocks{
		BaseResult: da.BaseResult{
			Code:     da.StatusSuccess,
			DAHeight: dataLayerHeight,
		},
		Blocks: blocks,
	}
}

// SubmitBlocks submits blocks to DA layer.
func (c *DataAvailabilityLayerClient) SubmitBlocks(ctx context.Context, blocks []*types.Block) da.ResultSubmitBlocks {
	c.logger.Info("submitting blocks to Sugondat Data Availability Layer", "blocks", blocks)
	blobs := make([]*Blob, len(blocks))
	for i, block := range blocks {
		data, err := block.MarshalBinary()
		if err != nil {
			return da.ResultSubmitBlocks{
				BaseResult: da.BaseResult{
					Code:    da.StatusError,
					Message: err.Error(),
				},
			}
		}
		blobs[i] = &Blob{Data: data}
	}

	dataLayerHeight, err := c.rpc.api.Submit(c.config.Namespace, blobs)
	if err != nil {
		return da.ResultSubmitBlocks{
			BaseResult: da.BaseResult{
				Code:    da.StatusError,
				Message: err.Error(),
			},
		}
	}

	c.logger.Debug("submitted blocks to Sugondat Data Availability Layer", "dataLayerHeight", dataLayerHeight)

	return da.ResultSubmitBlocks{
		BaseResult: da.BaseResult{
			Code:     da.StatusSuccess,
			DAHeight: dataLayerHeight,
		},
	}
}
