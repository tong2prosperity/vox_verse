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

    useEffect(() => {
        const service = new WebRTCService(user);
        setWebRTCService(service);

        service.setOnServerConnected((serverId) => {
            setTargetUserId(serverId);
        });

        service.setOnStreamCallback((stream, userId) => {
            if (remoteVideoRef.current) {
                remoteVideoRef.current.srcObject = stream;
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
