import 'package:flutter/material.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';
import '../../services/webrtc_services.dart';

class VideoChatPage extends StatefulWidget {
  @override
  _VideoChatPageState createState() => _VideoChatPageState();
}

class _VideoChatPageState extends State<VideoChatPage> {
  final _localRenderer = RTCVideoRenderer();
  final _remoteRenderer = RTCVideoRenderer();
  late WebRTCService _webRTCService;
  bool _isConnected = false;

  @override
  void initState() {
    super.initState();
    _initRenderers();
    _initWebRTC();
  }

  Future<void> _initRenderers() async {
    await _localRenderer.initialize();
    await _remoteRenderer.initialize();
  }

  void _initWebRTC() {
    final clientId = 'user-${DateTime.now().millisecondsSinceEpoch}';
    _webRTCService = WebRTCService(clientId);

    _webRTCService.onServerConnected = (serverId) {
      debugPrint('Connected to server: $serverId');
      setState(() => _isConnected = true);
    };

    _webRTCService.onRemoteStream = (stream) {
      setState(() {
        _remoteRenderer.srcObject = stream;
      });
    };
  }

  Future<void> _startCall() async {
    await _webRTCService.initializeMedia();
    await _webRTCService.startCall();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('视频聊天')),
      body: Column(
        children: [
          Expanded(
            child: Row(
              children: [
                Expanded(
                  child: RTCVideoView(_localRenderer),
                ),
                Expanded(
                  child: RTCVideoView(_remoteRenderer),
                ),
              ],
            ),
          ),
          Padding(
            padding: EdgeInsets.all(8.0),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                ElevatedButton(
                  onPressed: _isConnected ? _startCall : null,
                  child: Text('开始通话'),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  @override
  void dispose() {
    _localRenderer.dispose();
    _remoteRenderer.dispose();
    _webRTCService.dispose();
    super.dispose();
  }
}
