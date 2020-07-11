<template>
  <v-app>
    <v-app-bar app dark color="primary">
      <v-toolbar-title class="headline text-uppercase">
        <router-link to="/" class="logo white--text">Factorio Servers Statistics</router-link>
      </v-toolbar-title>
      <v-spacer></v-spacer>
    </v-app-bar>

    <v-content>
      <v-container fluid fill-height>
        <router-view />
      </v-container>
    </v-content>
  </v-app>
</template>

<style scoped>
  .container {
    padding: 24px;
  }

  .logo {
    text-decoration: none;
  }
</style>

<style>
  html {
    overflow-y: auto !important;
  }

  a:not(:hover) {
    text-decoration: none !important;
  }

  .rich-text img {
    margin: 0 0.5vw;
    display: inline-block;
    vertical-align: middle;
  }

  .rich-text {
    white-space: pre-wrap;
    display: inline-flex;
    align-items: center;
  }
</style>

<script>
  export default {
    name: 'App',
  };
</script>
