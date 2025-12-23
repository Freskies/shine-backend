from rest_framework import serializers
from members.models import Member

class MemberListSerializer(serializers.ModelSerializer):
    class Meta:
        model = Member
        fields = ('id', 'name')