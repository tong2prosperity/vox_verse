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
                    console.log('serverId:', this.serverId);
                    console.log('Connected to RTC server:', this.serverId);
                    if (this.onServerConnectedCallback && this.serverId) {
                        this.onServerConnectedCallback(this.serverId);
                    }
                    break;

                case 'answer':
                    // debug log
                    console.log('Received answer:', message);
                    
                    var payload1 = message.payload;
                    //console.log('serverId:', this.serverId, "from ", pylod.from);
                    
                    var pc0 = this.peerConnections.get(this.serverId!);
                    if (pc0) {
                        console.log('Setting remote description:', payload1.sdp);
                        await pc0.setRemoteDescription(new RTCSessionDescription(JSON.parse(payload1.sdp)));
                    }
                    
                    break;

                case 'ice_candidate':
                    var pylod = message.payload;
                    //if (pylod.from === this.serverId) {
                    console.log('Received ice candidate:', message);
                    var pc = this.peerConnections.get(this.serverId!);
                    if (pc) {
                        console.log('Adding ice candidate:', pylod.candidate);
                        await pc.addIceCandidate(new RTCIceCandidate(JSON.parse(pylod.candidate)));
                    }
                    //}
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
                console.log('Sending ice candidate:', event.candidate);
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

        pc.onicecandidateerror = (event) => {
            //console.log('ICE candidate error:', event.errorText, event.errorCode, event.address);
            console.log('ICE candidate error:', pc.iceConnectionState);
        };

        pc.ontrack = (event) => {
            if (this.onStreamCallback) {
                console.log('Received stream:', event.streams[0]);
                this.onStreamCallback(event.streams[0], this.serverId!);
            }
            
            // 处理音频track
            event.streams[0].getTracks().forEach(track => {
                console.log('Received track:', track.kind);
                if (track.kind === 'audio') {
                    console.log('Received audio track:', track.id);
                    this.handleAudioTrack(event.streams[0]);
                }
            });
        };

        pc.onicegatheringstatechange = (event) => {
            console.log('ICE gathering state changed:', pc.iceGatheringState);
        };

        pc.oniceconnectionstatechange = (event) => {
            console.log('ICE connection state changed:', pc.iceConnectionState);
        };

        pc.onconnectionstatechange = (event) => {
            console.log('Connection state changed:', pc.connectionState);
        };

        pc.onsignalingstatechange = (event) => {
            console.log('Signaling state changed:', pc.signalingState);
        };

        pc.ondatachannel = (event) => {
            console.log('Data channel received:', event.channel.label);
            const dataChannel = event.channel;
            
            // 设置数据通道事件处理
            dataChannel.onopen = () => {
                console.log('Data channel is open and ready to use');
            };
            
            dataChannel.onmessage = (event) => {
                console.log('Received message from data channel:', event.data);
                
                // 检查是否是文件数据
                if (event.data instanceof Blob) {
                    console.log('Received file data, size:', event.data.size);
                    this.handleFileReceived(event.data);
                } else if (typeof event.data === 'string') {
                    try {
                        const message = JSON.parse(event.data);
                        if (message.type === 'file-info') {
                            console.log('Received file info:', message);
                        }
                    } catch (e) {
                        console.log('Received text message:', event.data);
                    }
                }
            };
            
            dataChannel.onerror = (error) => {
                console.error('Data Channel Error:', error);
            };
            
            dataChannel.onclose = () => {
                console.log('The Data Channel is Closed');
            };
        };

        if (this.localStream) {
            this.localStream.getTracks().forEach(track => {
                if (this.localStream) {
                    console.log('Adding track:', track);
                    pc.addTrack(track, this.localStream);
                }
            });
        }

        this.peerConnections.set(this.serverId, pc);
        return pc;
    }

    // 处理音频track的方法
    private handleAudioTrack(stream: MediaStream) {
        console.log('处理音频流，轨道数量:', stream.getTracks().length);
        
        // 检查音频轨道
        const audioTracks = stream.getAudioTracks();
        console.log('音频轨道数量:', audioTracks.length);
        audioTracks.forEach(track => {
            console.log('音频轨道信息:', {
                id: track.id,
                enabled: track.enabled,
                muted: track.muted,
                readyState: track.readyState,
                contentHint: track.contentHint
            });
        });
        
        // 创建音频元素并播放
        const audioElement = document.createElement('audio');
        
        // 添加事件监听器以便调试
        audioElement.onloadedmetadata = () => console.log('音频元数据已加载');
        audioElement.oncanplay = () => console.log('音频可以开始播放');
        audioElement.onplay = () => console.log('音频开始播放');
        audioElement.onplaying = () => console.log('音频正在播放');
        audioElement.onwaiting = () => console.log('音频等待中');
        audioElement.onerror = (e) => console.error('音频播放错误:', e);
        
        // 设置属性
        audioElement.srcObject = stream;
        audioElement.autoplay = true;
        audioElement.controls = true; // 显示控件以便调试
        audioElement.style.position = 'fixed';
        audioElement.style.bottom = '10px';
        audioElement.style.right = '10px';
        audioElement.style.zIndex = '1000';
        
        // 确保添加到DOM
        document.body.appendChild(audioElement);
        
        // 尝试强制播放
        const playPromise = audioElement.play();
        if (playPromise !== undefined) {
            playPromise
                .then(() => console.log('音频播放成功启动'))
                .catch(error => {
                    console.error('自动播放失败:', error);
                    // 添加一个按钮让用户点击播放
                    const playButton = document.createElement('button');
                    playButton.textContent = '播放音频';
                    playButton.style.position = 'fixed';
                    playButton.style.bottom = '50px';
                    playButton.style.right = '10px';
                    playButton.style.zIndex = '1001';
                    playButton.onclick = () => {
                        audioElement.play()
                            .then(() => console.log('用户触发播放成功'))
                            .catch(err => console.error('用户触发播放失败:', err));
                    };
                    document.body.appendChild(playButton);
                });
        }
        
        console.log('音频元素已创建并尝试播放');
    }

    // 处理接收到的文件数据
    private handleFileReceived(fileData: Blob) {
        // 创建一个URL以便播放或下载
        const fileUrl = URL.createObjectURL(fileData);
        
        // 创建音频元素播放文件
        const audioElement = document.createElement('audio');
        audioElement.src = fileUrl;
        audioElement.controls = true;
        
        // 添加到DOM以便用户可以控制
        audioElement.style.position = 'fixed';
        audioElement.style.bottom = '80px';
        audioElement.style.right = '10px';
        audioElement.style.zIndex = '1000';
        document.body.appendChild(audioElement);
        
        // 添加下载按钮
        const downloadButton = document.createElement('button');
        downloadButton.textContent = '下载音频文件';
        downloadButton.style.position = 'fixed';
        downloadButton.style.bottom = '130px';
        downloadButton.style.right = '10px';
        downloadButton.style.zIndex = '1000';
        downloadButton.onclick = () => {
            const a = document.createElement('a');
            a.href = fileUrl;
            a.download = 'audio-file.opus'; // 或其他适当的文件名
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
        };
        document.body.appendChild(downloadButton);
        
        console.log('音频文件已准备好播放和下载');
    }

    public async startCall() {
        if (!this.serverId) {
            throw new Error('No RTC server assigned');
        }

        const pc = await this.createPeerConnection();
        const offer = await pc.createOffer();
        console.log('offer:', offer.sdp);
        await pc.setLocalDescription(offer);

        this.ws.send(JSON.stringify({
            type: 'offer',
            payload: {
                from: this.clientId,
                to: this.serverId,
                sdp: JSON.stringify(offer)
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
