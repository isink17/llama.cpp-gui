import { type KeyboardEvent, useState } from 'react';
import type { ChatStreamStatus } from '../lib/tauri/api';
import { type FailureView, getFailureView } from '../utils/failure';

export type ChatCardProps = {
  chatModel: string;
  chatPrompt: string;
  chatStatuses: ChatStreamStatus[];
  chatLog: string[];
  isStartingChat: boolean;
  isCancellingChat: boolean;
  canCancelChat: boolean;
  onChatModelChange: (value: string) => void;
  onChatPromptChange: (value: string) => void;
  onStartChat: () => void;
  onCancelChat: () => void;
  onUseAsPrompt?: (text: string) => void;
  onSendPrompt: () => void;
  onNavigateHistory: (direction: number) => void;
  onCancelGeneration: () => void;
};

const getToneForState = (value: string) => {
  switch (value.toLowerCase()) {
    case 'streaming':
      return 'info';
    case 'completed':
      return 'success';
    case 'cancelled':
      return 'neutral';
    case 'failed':
      return 'danger';
    default:
      return 'neutral';
  }
};

const formatStatusLabel = (value: string) =>
  value
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (character) => character.toUpperCase());

const getFailureViewForStatus = (status: ChatStreamStatus) => {
  if (status.error) {
    return getFailureView(status.error);
  }

  return {
    label: 'Failure',
    tone: 'danger' as const,
    detailClass: 'failure' as const,
  };
};

export function ChatCard({
  chatModel,
  chatPrompt,
  chatStatuses,
  chatLog,
  isStartingChat,
  isCancellingChat,
  canCancelChat,
  onChatModelChange,
  onChatPromptChange,
  onStartChat,
  onCancelChat,
  onUseAsPrompt,
  onSendPrompt,
  onNavigateHistory,
  onCancelGeneration,
}: ChatCardProps) {
  const [copiedFlag, setCopiedFlag] = useState(false);

  const handlePromptKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      onSendPrompt();
      return;
    }

    if (e.key === 'ArrowUp' && e.ctrlKey) {
      e.preventDefault();
      onNavigateHistory(-1);
      return;
    }

    if (e.key === 'ArrowDown' && e.ctrlKey) {
      e.preventDefault();
      onNavigateHistory(1);
      return;
    }

    if (e.key === 'Escape' && canCancelChat) {
      e.preventDefault();
      onCancelGeneration();
    }
  };

  const handleCopy = (text: string) => {
    void navigator.clipboard.writeText(text).then(() => {
      setCopiedFlag(true);
      setTimeout(() => setCopiedFlag(false), 1500);
    });
  };
  const activeStream = chatStatuses.find((item) => item.state === 'streaming');
  const chatSummaryLabel = activeStream
    ? `Streaming ${activeStream.model}`
    : chatStatuses.length
      ? 'Idle'
      : 'No streams';
  const chatSummaryTone = activeStream
    ? 'info'
    : chatStatuses.some((item) => item.state === 'failed')
      ? 'danger'
      : chatStatuses.some((item) => item.state === 'completed')
        ? 'success'
        : 'neutral';

  return (
    <article className="card chat-card">
      <h2>Chat stream</h2>
      <div className="status-summary">
        <span className={`status-badge ${chatSummaryTone}`}>{chatSummaryLabel}</span>
        <span className="hint">{chatStatuses.length} tracked</span>
      </div>
      <label>
        Model
        <input value={chatModel} onChange={(e) => onChatModelChange(e.target.value)} />
      </label>
      <label>
        Prompt
        <textarea
          value={chatPrompt}
          onChange={(e) => onChatPromptChange(e.target.value)}
          onKeyDown={handlePromptKeyDown}
          rows={3}
        />
      </label>
      <div className="row">
        <button onClick={() => void onStartChat()} disabled={isStartingChat}>
          {isStartingChat ? 'Starting...' : 'Start stream'}
        </button>
        <button onClick={() => void onCancelChat()} disabled={!canCancelChat || isCancellingChat}>
          {isCancellingChat ? 'Cancelling...' : 'Cancel stream'}
        </button>
      </div>
      <div className="panel">
        <div className="panel-header">
          <h3>Stream statuses</h3>
          <span className="hint">Live refresh updates</span>
        </div>
        <div className="entry-list">
          {chatStatuses.length ? (
            chatStatuses.map((status) => {
              const failureView =
                status.error || status.state === 'failed'
                  ? getFailureViewForStatus(status)
                  : null;

              return (
                <article className="entry-card" key={status.streamId}>
                  <div className="entry-card-header">
                    <div className="status-summary">
                      <span className={`status-badge ${getToneForState(status.state)}`}>
                        {formatStatusLabel(status.state)}
                      </span>
                      <strong>{status.model}</strong>
                    </div>
                    <div className="hint">{status.streamId}</div>
                  </div>
                  <div className="hint">
                    Bytes received: {status.bytesReceived.toLocaleString()}
                  </div>
                  {failureView ? (
                    <div className={`status-detail ${failureView.detailClass}`}>
                      <span className={`status-badge ${failureView.tone}`}>
                        {failureView.label}
                      </span>
                      <p className="status-detail-text">
                        {status.error ?? 'The stream ended in a failed state.'}
                      </p>
                    </div>
                  ) : null}
                </article>
              );
            })
          ) : (
            <p className="empty-state">No chat streams tracked yet.</p>
          )}
        </div>
      </div>
      {chatLog.length > 0 && (
        <div className="chat-message-block">
          <pre>{chatLog.join('')}</pre>
          <div className="chat-message-actions">
            <button
              className="secondary small"
              onClick={() => handleCopy(chatLog.join(''))}
            >
              {copiedFlag ? 'Copied!' : 'Copy'}
            </button>
            <button
              className="secondary small"
              onClick={() => onUseAsPrompt?.(chatLog.join(''))}
            >
              Use as prompt
            </button>
          </div>
        </div>
      )}
    </article>
  );
}
