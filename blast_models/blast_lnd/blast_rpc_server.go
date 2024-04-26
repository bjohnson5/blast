package main

import (
	"context"
	"encoding/hex"
	"errors"

	"github.com/lightningnetwork/lnd/lnrpc"

	pb "blast_lnd/blast_proto" // Import your generated proto file
)

type BlastRpcServer struct {
	pb.UnimplementedBlastRpcServer
	blast_lnd *BlastLnd
}

func (s *BlastRpcServer) StartNodes(ctx context.Context, request *pb.BlastStartRequest) (*pb.BlastStartResponse, error) {
	err := s.blast_lnd.start_nodes(int(request.NumNodes))
	response := &pb.BlastStartResponse{
		Success: err == nil,
	}
	return response, err
}

func (s *BlastRpcServer) GetSimLn(ctx context.Context, request *pb.BlastSimlnRequest) (*pb.BlastSimlnResponse, error) {
	response := &pb.BlastSimlnResponse{
		SimlnData: s.blast_lnd.simln_data,
	}
	return response, nil
}

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
		_, err := client.OpenChannel(ctx, req)
		if err != nil {
			err_val = err
		} else {
			err_val = nil
			response.Success = true
		}
	}

	return response, err_val
}

// TODO: implement this RPC function
func (s *BlastRpcServer) CloseChannel(ctx context.Context, request *pb.BlastCloseChannelRequest) (*pb.BlastCloseChannelResponse, error) {
	response := &pb.BlastCloseChannelResponse{
		Success: true,
	}
	return response, nil
}

func (s *BlastRpcServer) ConnectPeer(ctx context.Context, request *pb.BlastConnectRequest) (*pb.BlastConnectResponse, error) {
	err_val := errors.New("could not find node connection")
	response := &pb.BlastConnectResponse{
		Success: false,
	}

	if client, ok := s.blast_lnd.clients[request.Node]; ok {
		req := &lnrpc.ConnectPeerRequest{Addr: &lnrpc.LightningAddress{Pubkey: request.PeerPubKey, Host: request.PeerAddr}, Perm: false, Timeout: 5}
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
