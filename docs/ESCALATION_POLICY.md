# Escalation Policy

사용자에게 즉시 확인해야 하는 경우:

- 언어 문법이나 메모리 모델이 둘 이상의 합리적 방향으로 갈리고, 이후 구현 비용이 큰 경우
- public history rewrite, remote bookmark 이동, repo 삭제, force push가 필요한 경우
- GitHub repo visibility, license, package publishing처럼 외부 정책 결정이 필요한 경우
- 네트워크/API credential/secret이 필요한 작업에서 인증 상태가 불명확한 경우
- 검증 실패가 현재 작업 범위를 넘어선 기존 결함으로 보이는 경우

그 외에는 현재 문서, 테스트, 코드 구조를 기준으로 자율적으로 진행한다.
