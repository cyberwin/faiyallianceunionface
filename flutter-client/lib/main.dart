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

    final _companyIdCtrl = TextEditingController(text: "beauty_001");
  final _nameCtrl = TextEditingController(text: "张三");
  final _imgPathCtrl = TextEditingController(text: "/sdcard/face_imgs/zhangsan.jpg");
  final _thirdIdCtrl = TextEditingController(text: "member_123");
  final _thirdApiCtrl = TextEditingController(text: "https://third-server.com/face/callback");

  @override
  void initState() {
    super.initState();
    _checkHealth(); // 启动时检查服务连接
  }

  // 检查服务健康状态
  Future<void> _checkHealth() async {
    setState(() => _isLoading = true);
    try {
      final health = await _api.healthCheck();
      setState(() => _status = health);
    } catch (e) {
      setState(() => _status = "服务连接失败：$e");
    } finally {
      setState(() => _isLoading = false);
    }
  }

  // 添加公司配置
  Future<void> _addConfig() async {
    setState(() => _isLoading = true);
    try {
      final config = CompanyConfig(
        companyId: _companyIdCtrl.text.trim(),
        thirdPartyApi: _thirdApiCtrl.text.trim(),
        cacheExpireSeconds: 3600,
        createdAt: DateTime.now().millisecondsSinceEpoch,
      );
      await _api.addCompanyConfig(config);
      _showMsg("公司配置添加成功");
    } catch (e) {
      _showMsg("添加失败：$e");
    } finally {
      setState(() => _isLoading = false);
    }
  }

  // 注册人员
  Future<void> _registerPerson() async {
    setState(() => _isLoading = true);
    try {
      final req = RegisterReq(
        companyId: _companyIdCtrl.text.trim(),
        name: _nameCtrl.text.trim(),
        imgPath: _imgPathCtrl.text.trim(),
        thirdPartyId: _thirdIdCtrl.text.trim(),
      );
      await _api.registerPerson(req);
      _showMsg("人员注册成功");
    } catch (e) {
      _showMsg("注册失败：$e");
    } finally {
      setState(() => _isLoading = false);
    }
  }

  // 人脸比对+闸机指令
  Future<void> _verifyFace() async {
    setState(() => _isLoading = true);
    try {
      final companyId = _companyIdCtrl.text.trim();
      final resp = await _api.verifyFace(companyId);
      String gateMsg = resp.status == 9 ? "✅ 闸机允许开门" : "❌ 闸机拒绝开门";
      _showMsg("$gateMsg\n提示：${resp.message}\n请求ID：${resp.requestId}");
    } catch (e) {
      _showMsg("比对失败：$e");
    } finally {
      setState(() => _isLoading = false);
    }
  }

  // 显示提示消息
  void _showMsg(String msg) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(msg),
        duration: const Duration(seconds: 3),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text("东方仙盟人脸识别接口中心"),
        centerTitle: true,
      ),
      body: Stack(
        children: [
          SingleChildScrollView(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 服务状态
                Container(
                  padding: const EdgeInsets.all(8),
                  color: _status.contains("正常") ? Colors.green[100] : Colors.red[100],
                  child: Text(_status),
                ),
                const SizedBox(height: 20),

                // 公司配置区域
                const Text("一、公司配置", style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
                const SizedBox(height: 10),
                TextField(
                  controller: _companyIdCtrl,
                  decoration: const InputDecoration(labelText: "公司ID", hintText: "如 beauty_001"),
                ),
                TextField(
                  controller: _thirdApiCtrl,
                  decoration: const InputDecoration(labelText: "第三方回调API", hintText: "如 https://xxx.com/callback"),
                ),
                const SizedBox(height: 10),
                ElevatedButton(onPressed: _addConfig, child: const Text("添加公司配置")),
                const SizedBox(height: 30),

                // 人员注册区域
                const Text("二、人员注册", style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
                const SizedBox(height: 10),
                TextField(
                  controller: _nameCtrl,
                  decoration: const InputDecoration(labelText: "人员姓名"),
                ),
                TextField(
                  controller: _imgPathCtrl,
                  decoration: const InputDecoration(labelText: "图片路径", hintText: "如 /sdcard/face/zhangsan.jpg"),
                ),
                TextField(
                  controller: _thirdIdCtrl,
                  decoration: const InputDecoration(labelText: "第三方ID", hintText: "如 会员ID member_123"),
                ),
                const SizedBox(height: 10),
                ElevatedButton(onPressed: _registerPerson, child: const Text("注册人员")),
                const SizedBox(height: 30),

                // 人脸比对区域
                const Text("三、人脸比对", style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
                const SizedBox(height: 10),
                Center(
                  child: ElevatedButton(
                    onPressed: _verifyFace,
                    style: ElevatedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 60, vertical: 20),
                      fontSize: 20,
                      backgroundColor: Colors.blueAccent,
                    ),
                    child: const Text("开始人脸比对"),
                  ),
                ),
              ],
            ),
          ),

          // 加载中遮罩
          if (_isLoading)
            Container(
              color: Colors.black.withOpacity(0.5),
              child: const Center(
                child: SpinKitCircle(color: Colors.white, size: 60),
              ),
            ),
        ],
      ),
    );
  }
}
