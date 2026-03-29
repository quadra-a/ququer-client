#!/bin/bash
# ralph.sh
n=1
MAX_LOOPS=5 # 防止无限烧钱/死循环

while [ $n -le $MAX_LOOPS ]; do
  ts=$(date -Iseconds)
  echo "--- Loop $n: $ts ---"
  
  # 1. 核心改进：执行前清理（可选），确保每一轮都是轻量级上下文
  # ccd /clear  # 如果你的别名支持直接传参

  # 2. 核心改进：使用 timeout 防止单个任务无限挂起
  # 如果 ccd 5分钟没反应，强制杀掉并进入下一轮
  timeout 30min cat PROMPT.md | ccd
  
  # 检查 ccd 的退出状态
  RET_CODE=$?
  if [ $RET_CODE -eq 124 ]; then
    echo "ERROR: Loop $n timed out (10m). Recording failure..."
    echo "[$ts] Loop $n timed out during execution." >> failures.log
  elif [ $RET_CODE -ne 0 ]; then
    echo "ERROR: ccd crashed with exit code $RET_CODE"
    echo "[$ts] ccd crashed (exit $RET_CODE). Check system logs." >> failures.log
  fi

  # 3. 检查 TODO 是否完成
  if ! grep -q "\[ \]" TODO.md; then 
    echo "SUCCESS: All tasks in TODO.md finished!"
    break 
  fi

  # 4. 自动存档点：防止代码改乱了找不回来
  git add . && git commit -m "ralph: checkpoint loop $n"
  
  n=$((n + 1))
  sleep 2 # 给系统和 API 一点缓冲时间
done