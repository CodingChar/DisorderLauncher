<script lang="ts">
  import { providerLabel } from '../text'
  import type { AccountRecord } from '../types'
  import Avatar from './Avatar.svelte'

  interface Props {
    account: AccountRecord
  }

  let { account }: Props = $props()
</script>

<div class="badge">
  <Avatar />
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

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  strong {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  small {
    color: var(--text-secondary);
    font-size: 11px;
  }
</style>
