package main

import (
	"context"
	"fmt"

	"github.com/lightningnetwork/lnd/lnrpc"

	pb "blast_lnd/blast_proto" // Import your generated proto file
)

type BlastRpcServer struct {
	pb.UnimplementedBlastRpcServer
	blast_lnd *BlastLnd
}

func (s *BlastRpcServer) StartNodes(ctx context.Context, request *pb.BlastStartRequest) (*pb.BlastStartResponse, error) {
	s.blast_lnd.start_nodes(int(request.NumNodes))
	response := &pb.BlastStartResponse{
		Success: true,
	}
	return response, nil
}

func (s *BlastRpcServer) GetSimLn(ctx context.Context, request *pb.BlastSimlnRequest) (*pb.BlastSimlnResponse, error) {
	response := &pb.BlastSimlnResponse{
		SimlnData: s.blast_lnd.simln_data,
	}
	return response, nil
}

func (s *BlastRpcServer) GetPubKey(ctx context.Context, request *pb.BlastPubKeyRequest) (*pb.BlastPubKeyResponse, error) {
	if con, ok := s.blast_lnd.clients[request.Node]; ok {
		client := lnrpc.NewLightningClient(con)
		req := &lnrpc.GetInfoRequest{}
		ctx := context.Background()
		resp, err := client.GetInfo(ctx, req)
		if err != nil {
			fmt.Println("Error calling the node " + err.Error())
		}
		response := &pb.BlastPubKeyResponse{
			PubKey: resp.IdentityPubkey,
		}
		return response, nil
	}

	response := &pb.BlastPubKeyResponse{
		PubKey: "unknown",
	}
	return response, nil
}

func (s *BlastRpcServer) ListPeers(ctx context.Context, request *pb.BlastPeersRequest) (*pb.BlastPeersResponse, error) {
	if con, ok := s.blast_lnd.clients[request.Node]; ok {
		client := lnrpc.NewLightningClient(con)
		req := &lnrpc.ListPeersRequest{LatestError: true}
		ctx := context.Background()
		resp, err := client.ListPeers(ctx, req)
		if err != nil {
			fmt.Println("Error calling the node " + err.Error())
		}
		response := &pb.BlastPeersResponse{
			Peers: resp.String(),
		}
		return response, nil
	}

	response := &pb.BlastPeersResponse{
		Peers: "{}",
	}
	return response, nil
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
