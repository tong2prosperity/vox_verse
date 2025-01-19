import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';
import 'package:web_socket_client/web_socket_client.dart';

class WebRTCService {
  final String clientId;
  WebSocket? _ws;
  RTCPeerConnection? _peerConnection;
  MediaStream? _localStream;
  String? _serverId;

  Function(MediaStream stream)? onRemoteStream;
  Function(String serverId)? onServerConnected;

  WebRTCService(this.clientId) {
    _connectSignaling();
  }

  void _connectSignaling() {
    final uri = Uri.parse('ws://localhost:9527/ws/client');
    _ws = WebSocket(uri);

    _ws?.connection.listen((state) {
      print('WebSocket connection state: $state');

      if (state is Connected) {
        _ws?.send(jsonEncode({
          'type': 'client_connect',
          'payload': {'client_id': clientId}
        }));
      }
    });

    _ws?.messages.listen((message) {
      if (message is! String) return;

      final data = jsonDecode(message);

      debugPrint('Received message: $data');

      switch (data['type']) {
        case 'client_connected':
          _serverId = data['payload']['server_id'];
          onServerConnected?.call(_serverId!);
          break;

        case 'answer':
          _handleAnswer(data['payload']);
          break;

        case 'ice_candidate':
          _handleIceCandidate(data['payload']);
          break;
      }
    });
  }

  Future<void> startCall() async {
    if (_serverId == null) {
      throw Exception('No RTC server assigned');
    }

    _peerConnection = await _createPeerConnection();
    final offer = await _peerConnection!.createOffer();
    await _peerConnection!.setLocalDescription(offer);
    _peerConnection?.onAddTrack = onAddTrack;

    _ws?.send(jsonEncode({
      'type': 'offer',
      'payload': {'from': clientId, 'to': _serverId, 'sdp': offer.sdp}
    }));
  }

  Future<void> _handleAnswer(Map<String, dynamic> payload) async {
    final sdp = RTCSessionDescription(
      payload['sdp'],
      'answer',
    );
    await _peerConnection?.setRemoteDescription(sdp);
  }

  Future<void> _handleIceCandidate(Map<String, dynamic> payload) async {
    final candidate = RTCIceCandidate(
      payload['candidate']['candidate'],
      payload['candidate']['sdpMid'],
      payload['candidate']['sdpMLineIndex'],
    );
    await _peerConnection?.addCandidate(candidate);
  }

  Future<RTCPeerConnection> _createPeerConnection() async {
    final config = {
      'iceServers': [
        {'urls': 'stun:stun.l.google.com:19302'}
      ]
    };

    final pc = await createPeerConnection(config);

    pc.onIceCandidate = (candidate) {
      if (candidate == null) return;

      _ws?.send(jsonEncode({
        'type': 'ice_candidate',
        'payload': {
          'from': clientId,
          'to': _serverId,
          'candidate': candidate.toMap()
        }
      }));
    };

    pc.onTrack = (event) {
      if (event.streams.isNotEmpty) {
        onRemoteStream?.call(event.streams[0]);
      }
    };

    return pc;
  }

  Future<void> initializeMedia() async {
    _localStream = await navigator.mediaDevices
        .getUserMedia({'audio': true, 'video': true});

    _localStream?.getTracks().forEach((track) {
      _peerConnection?.addTrack(track, _localStream!);
    });
  }

  void dispose() {
    _peerConnection?.close();
    _ws?.close();
    _localStream?.dispose();
  }

  void onAddTrack(MediaStream stream, MediaStreamTrack track) {
    onRemoteStream?.call(stream);
  }
}
