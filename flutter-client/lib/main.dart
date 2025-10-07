import 'package:flutter/material.dart';
import 'api_client.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';

void main() => runApp(const MyApp());

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '东方仙盟人脸识别',
      theme: ThemeData(primarySwatch: Colors.blue),
      home: const HomePage(),
    );
  }
}

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  final ApiClient _api = ApiClient();
  String _status = "未连接服务";
  bool _isLoading = false;

  // 输入控制器
  final _companyIdCtrl = TextEditingController(text: "beauty_
