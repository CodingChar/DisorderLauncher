<script lang="ts">
  import { providerLabel } from '../text'
  import type { AccountRecord } from '../types'

  interface Props {
    account: AccountRecord
  }

  let { account }: Props = $props()
</script>

<div class="badge">
  <span class="avatar" aria-hidden="true">{account.label.charAt(0).toUpperCase()}</span>
  <span class="text">
    <strong>{account.label}</strong>
    <small>{providerLabel[account.provider]}</small>
  </span>
</div>

<style>
  .badge {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .avatar {
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--action);
    color: var(--action-text);
    font-family: var(--font-display);
    font-weight: 700;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  strong {
    font-size: 15px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  small {
    color: var(--text-secondary);
    font-size: 12px;
  }
</style>
