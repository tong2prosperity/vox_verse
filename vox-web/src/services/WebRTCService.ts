import { Message, User } from '../types/types';

export class WebRTCService {
    private ws: WebSocket;
    private peerConnections: Map<string, RTCPeerConnection> = new Map();
    private localStream: MediaStream | null = null;
    private onStreamCallback: ((stream: MediaStream, userId: string) => void) | null = null;
    private clientId: string | null = null;

    constructor(private user: User) {
        this.ws = new WebSocket('ws://localhost:9527/ws/client');
        this.setupWebSocketListeners();
    }

    private setupWebSocketListeners() {
        this.ws.onopen = () => {
            console.log('Connected to signaling server');
        };

        this.ws.onmessage = async (event) => {
            const message = JSON.parse(event.data);
            console.log('Received message:', message);

            switch (message.type) {
                case 'connected':
                    this.clientId = message.client_id;
                    // 连接后发送connect消息来请求分配RTC服务器
                    this.ws.send(JSON.stringify({
                        type: 'connect',
                        payload: {}
                    }));
                    break;
                    
                case 'server_assigned':
                    // 服务器分配完成，可以开始呼叫
                    console.log('RTC server assigned:', message.server_id);
                    break;

                case 'answer':
                    const pc = this.peerConnections.get(message.user_id);
                    if (pc) {
                        await pc.setRemoteDescription(new RTCSessionDescription(message.sdp));
                    }
                    break;

                case 'candidate':
                    const peerConnection = this.peerConnections.get(message.user_id);
                    if (peerConnection) {
                        await peerConnection.addIceCandidate(new RTCIceCandidate(message.candidate));
                    }
                    break;
            }
        };
    }

    private async createPeerConnection(targetUserId: string): Promise<RTCPeerConnection> {
        const pc = new RTCPeerConnection({
            iceServers: [
                { urls: 'stun:stun.l.google.com:19302' }
            ]
        });

        pc.onicecandidate = (event) => {
            if (event.candidate) {
                this.ws.send(JSON.stringify({
                    type: 'message',
                    payload: {
                        type: 'candidate',
                        user_id: targetUserId,
                        candidate: event.candidate
                    }
                }));
            }
        };

        pc.ontrack = (event) => {
            if (this.onStreamCallback) {
                this.onStreamCallback(event.streams[0], targetUserId);
            }
        };

        if (this.localStream) {
            this.localStream.getTracks().forEach(track => {
                if (this.localStream) {
                    pc.addTrack(track, this.localStream);
                }
            });
        }

        this.peerConnections.set(targetUserId, pc);
        return pc;
    }

    public async startCall(targetUserId: string) {
        const pc = await this.createPeerConnection(targetUserId);
        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);

        this.ws.send(JSON.stringify({
            type: 'message',
            payload: {
                type: 'calling',
                user_id: targetUserId,
                sdp: offer
            }
        }));
    }

    public async initializeMedia() {
        try {
            this.localStream = await navigator.mediaDevices.getUserMedia({
                video: true,
                audio: true
            });
            return this.localStream;
        } catch (error) {
            console.error('Error accessing media devices:', error);
            throw error;
        }
    }

    public setOnStreamCallback(callback: (stream: MediaStream, userId: string) => void) {
        this.onStreamCallback = callback;
    }

    public disconnect() {
        this.peerConnections.forEach(pc => pc.close());
        this.peerConnections.clear();
        if (this.localStream) {
            this.localStream.getTracks().forEach(track => track.stop());
        }
        this.ws.close();
    }
}
