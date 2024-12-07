import { Message, User } from '../types/types';
import io, { Socket } from 'socket.io-client';

export class WebRTCService {
    private socket: Socket;
    private peerConnections: Map<string, RTCPeerConnection> = new Map();
    private localStream: MediaStream | null = null;
    private onStreamCallback: ((stream: MediaStream, userId: string) => void) | null = null;

    constructor(private user: User) {
        this.socket = io('ws://localhost:8000');
        this.setupSocketListeners();
    }

    private setupSocketListeners() {
        this.socket.on('connect', () => {
            console.log('Connected to signaling server');
            this.socket.emit('join', this.user);
        });

        this.socket.on('offer', async (message: Message) => {
            const pc = await this.createPeerConnection(message.sender.id);
            await pc.setRemoteDescription(new RTCSessionDescription(message.data));
            const answer = await pc.createAnswer();
            await pc.setLocalDescription(answer);
            
            this.socket.emit('answer', {
                type: 'answer',
                sender: this.user,
                target: message.sender.id,
                data: answer
            });
        });

        this.socket.on('answer', async (message: Message) => {
            const pc = this.peerConnections.get(message.sender.id);
            if (pc) {
                await pc.setRemoteDescription(new RTCSessionDescription(message.data));
            }
        });

        this.socket.on('ice-candidate', async (message: Message) => {
            const pc = this.peerConnections.get(message.sender.id);
            if (pc) {
                await pc.addIceCandidate(new RTCIceCandidate(message.data));
            }
        });
    }

    private async createPeerConnection(targetUserId: string): Promise<RTCPeerConnection> {
        const pc = new RTCPeerConnection({
            iceServers: [
                { urls: 'stun:stun.l.google.com:19302' }
            ]
        });

        pc.onicecandidate = (event) => {
            if (event.candidate) {
                this.socket.emit('ice-candidate', {
                    type: 'ice-candidate',
                    sender: this.user,
                    target: targetUserId,
                    data: event.candidate
                });
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

        this.socket.emit('offer', {
            type: 'offer',
            sender: this.user,
            target: targetUserId,
            data: offer
        });
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
        this.socket.disconnect();
    }
}
