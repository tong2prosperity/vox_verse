import { Message, User } from '../types/types';

export class WebRTCService {
    private ws: WebSocket;
    private peerConnections: Map<string, RTCPeerConnection> = new Map();
    private localStream: MediaStream | null = null;
    private onStreamCallback: ((stream: MediaStream, userId: string) => void) | null = null;
    private onServerConnectedCallback: ((serverId: string) => void) | null = null;
    private clientId: string;
    private serverId: string | null = null;

    constructor(private user: User) {
        this.clientId = user.id;
        this.ws = new WebSocket('ws://localhost:9527/ws/client');
        this.setupWebSocketListeners();
    }

    private setupWebSocketListeners() {
        this.ws.onopen = () => {
            console.log('Connected to signaling server');
            // 发送ClientConnect消息
            this.ws.send(JSON.stringify({
                type: 'client_connect',
                payload: {
                    client_id: this.clientId
                }
            }));
        };

        this.ws.onmessage = async (event) => {
            const message = JSON.parse(event.data);
            console.log('Received message:', message);

            switch (message.type) {
                case 'client_connected':
                    this.serverId = message.payload.server_id;
                    console.log('Connected to RTC server:', this.serverId);
                    if (this.onServerConnectedCallback && this.serverId) {
                        this.onServerConnectedCallback(this.serverId);
                    }
                    break;

                case 'answer':
                    // debug log
                    console.log('Received answer:', message);
                    if (message.from === this.serverId) {
                        const pc = this.peerConnections.get(this.serverId!);
                        if (pc) {
                            await pc.setRemoteDescription(new RTCSessionDescription({
                                type: 'answer',
                                sdp: message.sdp
                            }));
                        }
                    }
                    break;

                case 'ice_candidate':
                    if (message.from === this.serverId) {
                        const pc = this.peerConnections.get(this.serverId!);
                        if (pc) {
                            await pc.addIceCandidate(new RTCIceCandidate(JSON.parse(message.candidate)));
                        }
                    }
                    break;
            }
        };
    }

    private async createPeerConnection(): Promise<RTCPeerConnection> {
        if (!this.serverId) {
            throw new Error('No RTC server assigned');
        }

        const pc = new RTCPeerConnection({
            iceServers: [
                { urls: 'stun:stun.l.google.com:19302' }
            ]
        });

        pc.onicecandidate = (event) => {
            if (event.candidate) {
                this.ws.send(JSON.stringify({
                    type: 'ice_candidate',
                    payload: {
                        from: this.clientId,
                        to: this.serverId,
                        candidate: JSON.stringify(event.candidate)
                    }
                }));
            }
        };

        pc.ontrack = (event) => {
            if (this.onStreamCallback) {
                this.onStreamCallback(event.streams[0], this.serverId!);
            }
        };

        if (this.localStream) {
            this.localStream.getTracks().forEach(track => {
                if (this.localStream) {
                    pc.addTrack(track, this.localStream);
                }
            });
        }

        this.peerConnections.set(this.serverId, pc);
        return pc;
    }

    public async startCall() {
        if (!this.serverId) {
            throw new Error('No RTC server assigned');
        }

        const pc = await this.createPeerConnection();
        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);

        this.ws.send(JSON.stringify({
            type: 'offer',
            payload: {
                from: this.clientId,
                to: this.serverId,
                sdp: offer.sdp
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
        if (this.serverId) {
            this.ws.send(JSON.stringify({
                type: 'client_disconnect',
                payload: {
                    client_id: this.clientId
                }
            }));
        }
        
        this.peerConnections.forEach(pc => pc.close());
        this.peerConnections.clear();
        if (this.localStream) {
            this.localStream.getTracks().forEach(track => track.stop());
        }
        this.ws.close();
    }

    public setOnServerConnected(callback: (serverId: string) => void) {
        this.onServerConnectedCallback = callback;
    }
}
