# VideoCard 썸네일/제목 개선 디자인

## 문제

1. **썸네일 잘림**: `aspect-[800/538]` + `object-cover`로 비율 안 맞는 이미지(16:9, 4:3 등)의 상하/좌우가 잘림
2. **제목 생략**: `line-clamp-2`로 긴 제목이 "..."으로 잘림, 전체 제목 확인 불가

## 솔루션: Blur Fill + Hover Expand

### 썸네일 — 블러 배경 채움

같은 이미지를 2겹으로 렌더링:

- **뒤쪽 (배경)**: `object-cover` + `blur(20px)` + `opacity-50` + `scale-110` — 컨테이너를 꽉 채우되 블러 처리
- **앞쪽 (실제)**: `object-contain` — 이미지 전체가 잘림 없이 표시

비율이 일치하는 이미지(800×538)는 contain이 cover와 동일하므로 블러 레이어가 보이지 않아 자연스럽다. 비율이 다른 이미지만 블러 배경이 여백을 채운다.

썸네일이 없는 경우(thumbnailPath === null) 기존 Play 아이콘 플레이스홀더 유지.

### 제목 — 호버 시 확장

- **기본**: `line-clamp-2` 유지 (현재와 동일)
- **호버**: 카드 하단 정보 영역을 `position: absolute`로 전환, `line-clamp` 해제하여 전체 제목 표시
- 호버 카드는 `z-index`를 올려 다른 카드 위에 표시
- `transition`으로 부드러운 전환
- 배우 목록(`truncate`)도 호버 시 전체 표시

### 호버 시각 효과

- 카드 border: `border-primary/50` (기존 호버 효과 유지)
- 그림자: `shadow-lg` 추가로 떠 있는 느낌
- 기존 Play 오버레이: 그대로 유지

## 변경 범위

- `src/components/library/VideoCard.tsx` — 썸네일 2겹 구조, 호버 확장 로직
- 다른 파일 변경 없음

## 구현 주의사항

- 블러 레이어는 `overflow-hidden` 컨테이너 안에서만 렌더 → 성능 영향 미미
- 호버 확장 시 카드 버튼의 클릭 영역이 변하지 않도록 주의 (onClick 핸들러 유지)
- `loading="lazy"` 유지 — 블러 배경 이미지도 동일한 src이므로 브라우저 캐시 활용
