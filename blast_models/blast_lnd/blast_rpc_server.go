package main

import (
	"context"
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

	if con, ok := s.blast_lnd.clients[request.Node]; ok {
		client := lnrpc.NewLightningClient(con)
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

	if con, ok := s.blast_lnd.clients[request.Node]; ok {
		client := lnrpc.NewLightningClient(con)
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

func (s *BlastRpcServer) WalletBalance(ctx context.Context, request *pb.BlastRpcRequest) (*pb.BlastRpcResponse, error) {
	response := &pb.BlastRpcResponse{
		Response: "Unimplemented",
	}
	return response, nil
}

func (s *BlastRpcServer) ChannelBalance(ctx context.Context, request *pb.BlastRpcRequest) (*pb.BlastRpcResponse, error) {
	response := &pb.BlastRpcResponse{
		Response: "Unimplemented",
	}
	return response, nil
}

func (s *BlastRpcServer) ListChannels(ctx context.Context, request *pb.BlastRpcRequest) (*pb.BlastRpcResponse, error) {
	response := &pb.BlastRpcResponse{
		Response: "Unimplemented",
	}
	return response, nil
}

func (s *BlastRpcServer) OpenChannel(ctx context.Context, request *pb.BlastRpcRequest) (*pb.BlastRpcResponse, error) {
	response := &pb.BlastRpcResponse{
		Response: "Unimplemented",
	}
	return response, nil
}

func (s *BlastRpcServer) CloseChannel(ctx context.Context, request *pb.BlastRpcRequest) (*pb.BlastRpcResponse, error) {
	response := &pb.BlastRpcResponse{
		Response: "Unimplemented",
	}
	return response, nil
}

func (s *BlastRpcServer) ConnectPeer(ctx context.Context, request *pb.BlastRpcRequest) (*pb.BlastRpcResponse, error) {
	response := &pb.BlastRpcResponse{
		Response: "Unimplemented",
	}
	return response, nil
}

func (s *BlastRpcServer) DisconnectPeer(ctx context.Context, request *pb.BlastRpcRequest) (*pb.BlastRpcResponse, error) {
	response := &pb.BlastRpcResponse{
		Response: "Unimplemented",
	}
	return response, nil
}
