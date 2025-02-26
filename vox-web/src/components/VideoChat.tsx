import React, { useEffect, useRef, useState } from 'react';
import { Box, Button, Container, Grid, Paper, Typography } from '@mui/material';
import { WebRTCService } from '../services/WebRTCService';
import { User } from '../types/types';

interface VideoChatProps {
    user: User;
    targetUserId?: string;
}

export const VideoChat: React.FC<VideoChatProps> = ({ user }) => {
    const [webRTCService, setWebRTCService] = useState<WebRTCService | null>(null);
    const [targetUserId, setTargetUserId] = useState<string | null>(null);
    const localVideoRef = useRef<HTMLVideoElement>(null);
    const remoteVideoRef = useRef<HTMLVideoElement>(null);
    const remoteAudioRef = useRef<HTMLAudioElement>(null);
    const [audioStream, setAudioStream] = useState<MediaStream | null>(null);

    useEffect(() => {
        const service = new WebRTCService(user);
        setWebRTCService(service);

        service.setOnServerConnected((serverId) => {
            setTargetUserId(serverId);
        });

        service.setOnStreamCallback((stream, userId) => {
            console.log('收到流，轨道类型:', stream.getTracks().map(t => t.kind).join(', '));
            
            // 处理视频流
            if (remoteVideoRef.current) {
                remoteVideoRef.current.srcObject = stream;
            }
            
            // 处理音频流
            if (stream.getAudioTracks().length > 0) {
                console.log('检测到音频轨道，设置到音频元素');
                setAudioStream(stream);
                if (remoteAudioRef.current) {
                    remoteAudioRef.current.srcObject = stream;
                    // 尝试播放
                    remoteAudioRef.current.play()
                        .then(() => console.log('组件内音频播放成功'))
                        .catch(err => console.error('组件内音频播放失败:', err));
                }
            }
        });

        const initializeVideo = async () => {
            try {
                const stream = await service.initializeMedia();
                if (localVideoRef.current) {
                    localVideoRef.current.srcObject = stream;
                }
            } catch (error) {
                console.error('Failed to initialize media:', error);
            }
        };

        initializeVideo();

        return () => {
            service.disconnect();
        };
    }, [user]);

    const handleStartCall = () => {
        if (webRTCService) {
            webRTCService.startCall();
        }
    };

    const handlePlayAudio = () => {
        if (remoteAudioRef.current && audioStream) {
            console.log('手动播放音频');
            remoteAudioRef.current.play()
                .then(() => console.log('手动播放成功'))
                .catch(err => console.error('手动播放失败:', err));
        } else {
            console.log('没有音频流或音频元素不存在');
        }
    };

    return (
        <Container maxWidth="lg">
            <Box sx={{ flexGrow: 1, mt: 4 }}>
                <Grid container spacing={3}>
                    <Grid item xs={12} md={6}>
                        <Paper elevation={3} sx={{ p: 2 }}>
                            <Typography variant="h6" gutterBottom>
                                Local Video
                            </Typography>
                            <video
                                ref={localVideoRef}
                                autoPlay
                                playsInline
                                muted
                                style={{ width: '100%', maxHeight: '400px' }}
                            />
                        </Paper>
                    </Grid>
                    <Grid item xs={12} md={6}>
                        <Paper elevation={3} sx={{ p: 2 }}>
                            <Typography variant="h6" gutterBottom>
                                Remote Video
                            </Typography>
                            <video
                                ref={remoteVideoRef}
                                autoPlay
                                playsInline
                                style={{ width: '100%', maxHeight: '400px' }}
                            />
                            {/* 添加音频元素用于调试 */}
                            <audio 
                                ref={remoteAudioRef}
                                autoPlay
                                controls
                                style={{ width: '100%', marginTop: '10px' }}
                            />
                            <Button 
                                variant="outlined" 
                                color="secondary" 
                                onClick={handlePlayAudio}
                                sx={{ mt: 1 }}
                            >
                                手动播放音频
                            </Button>
                        </Paper>
                    </Grid>
                    <Grid item xs={12}>
                        <Box sx={{ display: 'flex', justifyContent: 'center', mt: 2 }}>
                            <Button
                                variant="contained"
                                color="primary"
                                onClick={handleStartCall}
                                disabled={!targetUserId}
                            >
                                {targetUserId ? 'Start Call' : 'Waiting for server...'}
                            </Button>
                        </Box>
                    </Grid>
                </Grid>
            </Box>
        </Container>
    );
};
