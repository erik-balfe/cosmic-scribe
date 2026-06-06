<script>
  let { options = [], value = $bindable(), id = undefined } = $props();
</script>

<select {id} bind:value>
  {#each options as opt (opt.value)}
    <option value={opt.value}>{opt.label}</option>
  {/each}
</select>