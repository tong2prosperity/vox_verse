export interface User {
    id: string;
    name: string;
}

export interface Message {
    type: 'join' | 'leave' | 'offer' | 'answer' | 'ice-candidate';
    sender: User;
    target?: string;
    data?: any;
}

export interface PeerConnection {
    peer: RTCPeerConnection;
    stream?: MediaStream;
}
