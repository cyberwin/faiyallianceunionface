import 'dart:convert';
import 'package:http/http.dart' as http;
import 'package:json_annotation/json_annotation.dart';

part 'api_client.g.dart';

// 公司配置模型
@JsonSerializable()
class CompanyConfig {
  final String companyId;
  final String thirdPartyApi;
  final int cacheExpireSeconds;
  final int createdAt;

  CompanyConfig({
    required this.companyId,
    required this.thirdPartyApi,
    required this.cacheExpireSeconds,
    required this.createdAt,
  });

  factory CompanyConfig.fromJson(Map<String, dynamic> json) =>
      _$CompanyConfigFromJson(json);
  Map<String, dynamic> toJson() => _$CompanyConfigToJson(this);
}

// 注册请求模型
@JsonSerializable()
class RegisterReq {
  final String companyId;
  final String name;
  final String imgPath;
  final String thirdPartyId;

  RegisterReq({
    required this.companyId,
    required this.name,
    required this.imgPath,
    required this.thirdPartyId,
  });

  factory RegisterReq.fromJson(Map<String, dynamic> json) =>
      _$RegisterReqFromJson(json);
  Map<String, dynamic> toJson() => _$RegisterReqToJson(this);
}

// 闸机回调结果模型
@JsonSerializable()
class ThirdPartyResp {
  final int status;
  final String message;
  final String requestId;

  ThirdPartyResp({
    required this.status,
    required this.message,
    required this.requestId,
  });

  factory ThirdPartyResp.fromJson(Map<String, dynamic> json) =>
      _$ThirdPartyRespFromJson(json);
  Map<String, dynamic> toJson() => _$ThirdPartyRespToJson(this);
}

// API统一响应模型
@JsonSerializable(genericArgumentFactories: true)
class ApiResp<T> {
  final T? data;
  final String? message;
  final int? code;

  ApiResp({this.data, this.message, this.code});

  factory ApiResp.fromJson(
    Map<String, dynamic> json,
    T Function(Object?) fromJsonT,
  ) => _$ApiRespFromJson(json, fromJsonT);
}

// API客户端
class ApiClient {
  final String baseUrl;
  final http.Client _client;

  ApiClient({this.baseUrl = "http://localhost:8080"}) : _client = http.Client();

  // 健康检查
  Future<String> healthCheck() async {
    final resp = await _client.get(Uri.parse("$baseUrl/health"));
    if (resp.statusCode != 200) throw Exception("服务未启动");
    return resp.body;
  }

  // 添加公司配置
  Future<void> addCompanyConfig(CompanyConfig config) async {
    final resp = await _client.post(
      Uri.parse("$baseUrl/config/company"),
      headers: {"Content-Type": "application/json"},
      body: jsonEncode(config.toJson()),
    );
    final apiResp = ApiResp.fromJson(jsonDecode(resp.body), (data) => null);
    if (apiResp.code != null) throw Exception(apiResp.message);
  }

  // 注册人员
  Future<void> registerPerson(RegisterReq req) async {
    final resp = await _client.post(
      Uri.parse("$baseUrl/register"),
      headers: {"Content-Type": "application/json"},
      body: jsonEncode(req.toJson()),
    );
    final apiResp = ApiResp.fromJson(jsonDecode(resp.body), (data) => null);
    if (apiResp.code != null) throw Exception(apiResp.message);
  }

  // 人脸比对+闸机指令
  Future<ThirdPartyResp> verifyFace(String companyId) async {
    final resp = await _client.post(
      Uri.parse("$baseUrl/verify/$companyId"),
      headers: {"Content-Type": "application/json"},
    );
    final apiResp = ApiResp.fromJson(
      jsonDecode(resp.body),
      (data) => ThirdPartyResp.fromJson(data as Map<String, dynamic>),
    );
    if (apiResp.code != null) throw Exception(apiResp.message);
    return apiResp.data!;
  }
}
