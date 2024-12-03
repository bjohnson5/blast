package main

import (
	"context"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"strconv"
	"strings"

	"github.com/lightningnetwork/lnd/lnrpc"

	pb "blast_lnd/blast_proto"
)

// The file to save the current channels data to
const CHANNEL_SUFFIX string = "_channels.json"

// The RPC server that the blast framework will connect to
type BlastRpcServer struct {
	pb.UnimplementedBlastRpcServer
	blast_lnd *BlastLnd
}

// Start a certain number of nodes
func (s *BlastRpcServer) StartNodes(ctx context.Context, request *pb.BlastStartRequest) (*pb.BlastStartResponse, error) {
	err := s.blast_lnd.start_nodes(int(request.NumNodes))
	response := &pb.BlastStartResponse{
		Success: err == nil,
	}
	return response, err
}

// Get the sim-ln data for this model
func (s *BlastRpcServer) GetSimLn(ctx context.Context, request *pb.BlastSimlnRequest) (*pb.BlastSimlnResponse, error) {
	response := &pb.BlastSimlnResponse{
		SimlnData: s.blast_lnd.simln_data,
	}
	return response, nil
}

// Blast requests the pub key of a node that is controlled by this model -- look up the node's RPC client and pass the request through to LND -- Blast -> Model -> Node
func (s *BlastRpcServer) GetPubKey(ctx context.Context, request *pb.BlastPubKeyRequest) (*pb.BlastPubKeyResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastPubKeyResponse{
		PubKey: "",
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.GetInfoRequest{}
		ctx := context.Background()
		resp, err := client.GetInfo(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.PubKey = resp.IdentityPubkey
		}
	}

	return response, err_val
}

// Blast requests the list of peers for a node that is controlled by this model -- look up the node's RPC client and pass the request through to LND -- Blast -> Model -> Node
func (s *BlastRpcServer) ListPeers(ctx context.Context, request *pb.BlastPeersRequest) (*pb.BlastPeersResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastPeersResponse{
		Peers: "{}",
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.ListPeersRequest{LatestError: true}
		ctx := context.Background()
		resp, err := client.ListPeers(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Peers = resp.String()
		}
	}

	return response, err_val
}

// Blast requests the wallet balance of a node that is controlled by this model -- look up the node's RPC client and pass the request through to LND -- Blast -> Model -> Node
func (s *BlastRpcServer) WalletBalance(ctx context.Context, request *pb.BlastWalletBalanceRequest) (*pb.BlastWalletBalanceResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastWalletBalanceResponse{
		Balance: "",
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.WalletBalanceRequest{}
		ctx := context.Background()
		resp, err := client.WalletBalance(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Balance = resp.String()
		}
	}

	return response, err_val
}

// Blast requests the channel balance of a node that is controlled by this model -- look up the node's RPC client and pass the request through to LND -- Blast -> Model -> Node
func (s *BlastRpcServer) ChannelBalance(ctx context.Context, request *pb.BlastChannelBalanceRequest) (*pb.BlastChannelBalanceResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastChannelBalanceResponse{
		Balance: "",
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.ChannelBalanceRequest{}
		ctx := context.Background()
		resp, err := client.ChannelBalance(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Balance = resp.String()
		}
	}

	return response, err_val
}

// Blast requests the list of channels for a node that is controlled by this model -- look up the node's RPC client and pass the request through to LND -- Blast -> Model -> Node
func (s *BlastRpcServer) ListChannels(ctx context.Context, request *pb.BlastListChannelsRequest) (*pb.BlastListChannelsResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastListChannelsResponse{
		Channels: "",
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.ListChannelsRequest{}
		ctx := context.Background()
		resp, err := client.ListChannels(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Channels = resp.String()
		}
	}

	return response, err_val
}

// Blast requests that a node controlled by this model opens a channel -- look up the node's RPC client and pass the request through to LND -- Blast -> Model -> Node
func (s *BlastRpcServer) OpenChannel(ctx context.Context, request *pb.BlastOpenChannelRequest) (*pb.BlastOpenChannelResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastOpenChannelResponse{
		Success: false,
	}

	nodePubHex, err := hex.DecodeString(request.PeerPubKey)
	if err != nil {
		err_val = err
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.OpenChannelRequest{
			NodePubkey:         nodePubHex,
			LocalFundingAmount: request.Amount,
			PushSat:            request.PushAmout,
		}
		ctx := context.Background()
		status_client, err := client.OpenChannel(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Success = true
			go func() {
				for {
					rpcUpdate, err := status_client.Recv()
					if err != nil {
						return
					}

					switch rpcUpdate.Update.(type) {
					case *lnrpc.OpenStatusUpdate_ChanPending:
					case *lnrpc.OpenStatusUpdate_ChanOpen:
						s.blast_lnd.open_channels[strconv.Itoa(int(request.ChannelId))] = ChannelPoint{Source: request.Node, Dest: request.PeerPubKey, FundingTxid: rpcUpdate.GetChanOpen().ChannelPoint.GetFundingTxidBytes(), OutputIndex: rpcUpdate.GetChanOpen().ChannelPoint.OutputIndex}
						return
					case *lnrpc.OpenStatusUpdate_PsbtFund:
					}
				}
			}()
		}
	}

	return response, err_val
}

// Blast requests that a node controlled by this model closes a channel -- look up the node's RPC client and pass the request through to LND -- Blast -> Model -> Node
func (s *BlastRpcServer) CloseChannel(ctx context.Context, request *pb.BlastCloseChannelRequest) (*pb.BlastCloseChannelResponse, error) {
	err_val := errors.New("could not find open channel")
	response := &pb.BlastCloseChannelResponse{
		Success: false,
	}

	var chan_point lnrpc.ChannelPoint
	if val, ok := s.blast_lnd.open_channels[strconv.Itoa(int(request.ChannelId))]; ok {
		funtx := lnrpc.ChannelPoint_FundingTxidBytes{FundingTxidBytes: val.FundingTxid}
		chan_point = lnrpc.ChannelPoint{FundingTxid: &funtx, OutputIndex: val.OutputIndex}
	} else {
		return response, err_val
	}

	err_val = errors.New("could not find node connection")
	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.CloseChannelRequest{ChannelPoint: &chan_point}
		ctx := context.Background()
		_, err := client.CloseChannel(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Success = true
			delete(s.blast_lnd.open_channels, strconv.Itoa(int(request.ChannelId)))
		}
	}

	return response, err_val
}

// Create a comma separated list of open channels that this model has control over
func (s *BlastRpcServer) GetModelChannels(ctx context.Context, request *pb.BlastGetModelChannelsRequest) (*pb.BlastGetModelChannelsResponse, error) {
	if len(s.blast_lnd.open_channels) == 0 {
		response := &pb.BlastGetModelChannelsResponse{
			Channels: "",
		}
		return response, nil
	}

	var sb strings.Builder
	for key, value := range s.blast_lnd.open_channels {
		sb.WriteString(fmt.Sprintf("%s: %s -> %s,", key, value.Source, value.Dest))
	}

	result := sb.String()
	if len(result) > 0 {
		result = result[:len(result)-1]
	}

	response := &pb.BlastGetModelChannelsResponse{
		Channels: result,
	}
	return response, nil
}

// Blast requests that a node controlled by this model connects to a peer -- look up the node's RPC client and pass the request through to LND -- Blast -> Model -> Node
func (s *BlastRpcServer) ConnectPeer(ctx context.Context, request *pb.BlastConnectRequest) (*pb.BlastConnectResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastConnectResponse{
		Success: false,
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.ConnectPeerRequest{Addr: &lnrpc.LightningAddress{Pubkey: request.PeerPubKey, Host: request.PeerAddr}, Perm: true, Timeout: 5}
		ctx := context.Background()
		_, err := client.ConnectPeer(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Success = true
		}
	}

	return response, err_val
}

// Blast requests that a node controlled by this model disconnects from a peer -- look up the node's RPC client and pass the request through to LND -- Blast -> Model -> Node
func (s *BlastRpcServer) DisconnectPeer(ctx context.Context, request *pb.BlastDisconnectRequest) (*pb.BlastDisconnectResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastDisconnectResponse{
		Success: false,
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.DisconnectPeerRequest{PubKey: request.PeerPubKey}
		ctx := context.Background()
		_, err := client.DisconnectPeer(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Success = true
		}
	}

	return response, err_val
}

// Get a BTC address for a node
func (s *BlastRpcServer) GetBtcAddress(ctx context.Context, request *pb.BlastBtcAddressRequest) (*pb.BlastBtcAddressResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastBtcAddressResponse{
		Address: "",
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.NewAddressRequest{Type: 4}
		ctx := context.Background()
		resp, err := client.NewAddress(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Address = resp.Address
		}
	}

	return response, err_val
}

// Get the listen address for a node
func (s *BlastRpcServer) GetListenAddress(ctx context.Context, request *pb.BlastListenAddressRequest) (*pb.BlastListenAddressResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastListenAddressResponse{
		Address: "",
	}

	if addr, ok := s.blast_lnd.listen_addresses[request.Node]; ok {
		err_val = nil
		response.Address = addr
	}

	return response, err_val
}

// Shutdown the nodes
func (s *BlastRpcServer) StopModel(ctx context.Context, request *pb.BlastStopModelRequest) (*pb.BlastStopModelResponse, error) {
	for _, client := range s.blast_lnd.clients {
		req := &lnrpc.StopRequest{}
		ctx := context.Background()
		client.StopDaemon(ctx, req)
	}

	response := &pb.BlastStopModelResponse{Success: true}
	s.blast_lnd.shutdown_ch <- struct{}{}
	return response, nil
}

// Load a previous state of this model
func (s *BlastRpcServer) Load(ctx context.Context, request *pb.BlastLoadRequest) (*pb.BlastLoadResponse, error) {
	response := &pb.BlastLoadResponse{
		Success: false,
	}

	homeDir, err := os.UserHomeDir()
	if err != nil {
		return response, err
	}

	sim_dir := homeDir + "/" + SIM_DIR + "/" + request.Sim + "/" + MODEL_NAME + "/"

	err = s.blast_lnd.load_nodes(sim_dir + request.Sim + ".tar.gz")
	if err != nil {
		return response, err
	}

	err = s.blast_lnd.load_channels(sim_dir + request.Sim + CHANNEL_SUFFIX)
	if err != nil {
		return response, err
	}

	response.Success = true

	return response, err
}

// Save this models current state
func (s *BlastRpcServer) Save(ctx context.Context, request *pb.BlastSaveRequest) (*pb.BlastSaveResponse, error) {
	response := &pb.BlastSaveResponse{
		Success: false,
	}

	homeDir, err := os.UserHomeDir()
	if err != nil {
		return response, err
	}

	sim_dir := homeDir + "/" + SIM_DIR + "/" + request.Sim + "/" + MODEL_NAME + "/"

	if _, err := os.Stat(sim_dir); os.IsNotExist(err) {
		os.MkdirAll(sim_dir, 0700)
	}

	sim_archive, err := os.Create(sim_dir + request.Sim + ".tar.gz")
	if err != nil {
		return response, err
	}

	err = Tar(s.blast_lnd.data_dir, sim_archive)
	if err != nil {
		return response, err
	}

	err = s.blast_lnd.save_channels(sim_dir + request.Sim + CHANNEL_SUFFIX)
	if err != nil {
		return response, err
	}

	response.Success = true
	return response, nil
}
